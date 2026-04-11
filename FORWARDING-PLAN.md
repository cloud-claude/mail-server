# Plan: Account-Level Email Forwarding UI for Stalwart

## Goal

Add a "Forward to" field on the account edit page in Stalwart's webadmin.
When an admin sets a forwarding address, the system auto-generates a trusted Sieve script
that handles forwarding for all configured accounts. No Sieve knowledge required.

---

## How It Works

### User Experience

Admin goes to **Manage > Directory > Accounts > Edit Account** and sees:

```
Email:           [ oz@oznakash.com        ]
Forward to:      [ oznakash@gmail.com     ]   ← new field
Keep local copy: [✓]                          ← new checkbox
Password:        [ ••••••••               ]
```

Save. Done. Emails to `oz@oznakash.com` are forwarded to `oznakash@gmail.com`.

### What Happens on Save

```
1. Webadmin saves forwarding config as settings keys:
   POST /api/manage/settings
   → forwarding.accounts.oz@oznakash.com.address = "oznakash@gmail.com"
   → forwarding.accounts.oz@oznakash.com.keep-copy = "true"

2. Webadmin reads ALL forwarding configs:
   GET /api/manage/settings/list?prefix=forwarding.accounts

3. Webadmin generates a trusted Sieve script from all entries:
   require ["envelope", "copy"];
   if envelope :is "to" "oz@oznakash.com" { redirect :copy "oznakash@gmail.com"; }
   if envelope :is "to" "oz@naka.sh" { redirect :copy "oznakash@gmail.com"; }

4. Webadmin saves the script as a setting:
   POST /api/manage/settings
   → sieve.trusted.scripts.auto-forward.contents = <generated script>

5. One-time: ensures session.data.script = "auto-forward"
```

### Data Flow Diagram

```
┌──────────────┐     ┌───────────────────┐     ┌─────────────────────┐
│  Admin UI    │     │  Settings API     │     │  Stalwart SMTP      │
│              │     │  (RocksDB)        │     │                     │
│ Forward to:  │────►│                   │     │                     │
│ gmail.com    │     │ forwarding.       │     │                     │
│              │     │  accounts.        │     │                     │
│ [Save]       │     │   oz@x.com.       │     │                     │
│   │          │     │    address=gmail   │     │                     │
│   │          │     │    keep-copy=true  │     │                     │
│   │          │     │                   │     │                     │
│   ▼          │     │ sieve.trusted.    │     │ session.data.script │
│ Generate     │────►│  scripts.         │────►│  = "auto-forward"   │
│ Sieve script │     │   auto-forward.   │     │                     │
│              │     │    contents=...   │     │ Runs on every       │
│              │     │                   │     │ inbound email       │
└──────────────┘     └───────────────────┘     └─────────────────────┘
```

---

## Data Storage

All data is stored as Stalwart settings keys (persisted in RocksDB). No schema changes.

### Forwarding Config Keys

```
forwarding.accounts.<email>.address    = "<forwarding email>"
forwarding.accounts.<email>.keep-copy  = "true" | "false"
```

### Generated Script Key

```
sieve.trusted.scripts.auto-forward.contents = "<sieve script>"
```

### Pipeline Trigger Key (one-time setup)

```
session.data.script = "auto-forward"
```

### Restart Behavior

All three are persisted in RocksDB. On restart:
- Settings keys survive (RocksDB)
- The trusted script survives (it's a setting)
- `session.data.script` survives (it's a setting)
- The webadmin ZIP is re-extracted from blob storage (stateless, no data in it)

**Nothing is lost on restart.**

---

## Sieve Script Generation

### Input

Array of forwarding configs read from settings:

```json
[
  {"email": "oz@oznakash.com", "address": "oznakash@gmail.com", "keepCopy": true},
  {"email": "oz@naka.sh", "address": "oznakash@gmail.com", "keepCopy": true}
]
```

### Output

```sieve
require ["envelope", "copy"];

if envelope :is "to" "oz@oznakash.com" {
    redirect :copy "oznakash@gmail.com";
}

if envelope :is "to" "oz@naka.sh" {
    redirect :copy "oznakash@gmail.com";
}
```

### Rules

- If `keepCopy = true`: use `redirect :copy "address";` (keeps local copy)
- If `keepCopy = false`: use `redirect "address"; stop;` (forward only)
- If `keepCopy` is mixed across accounts: include `"copy"` in require
- If no accounts have forwarding: set script contents to empty string
- Script is regenerated from ALL forwarding configs on every save (idempotent)

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No forwarding configured | Empty script or script deleted |
| Script already exists from manual setup | Overwritten. Admin warned on first use. |
| `session.data.script` set to different script | Check on save. If set to something else, append or warn. |
| Server restart | All settings in RocksDB. Nothing lost. |
| Multiple admins editing simultaneously | Last write wins (settings API is atomic per request) |
| Email address with special chars | Sieve uses quoted strings, handles this natively |
| Admin deletes the account | Forwarding keys must be cleaned up on account deletion |
| Very large number of accounts | Single script with many if-blocks. Sieve handles this fine. |

---

## Test Cases

### Test 1: Single Account with Copy

```
Input:  [{"email": "oz@oznakash.com", "address": "oznakash@gmail.com", "keepCopy": true}]

Expected output:
  require ["envelope", "copy"];

  if envelope :is "to" "oz@oznakash.com" {
      redirect :copy "oznakash@gmail.com";
  }
```

### Test 2: Multiple Accounts

```
Input:  [
  {"email": "oz@oznakash.com", "address": "oznakash@gmail.com", "keepCopy": true},
  {"email": "oz@naka.sh", "address": "oznakash@gmail.com", "keepCopy": true}
]

Expected output:
  require ["envelope", "copy"];

  if envelope :is "to" "oz@oznakash.com" {
      redirect :copy "oznakash@gmail.com";
  }

  if envelope :is "to" "oz@naka.sh" {
      redirect :copy "oznakash@gmail.com";
  }
```

### Test 3: Forward Only (No Local Copy)

```
Input:  [{"email": "oz@naka.sh", "address": "oznakash@gmail.com", "keepCopy": false}]

Expected output:
  require ["envelope"];

  if envelope :is "to" "oz@naka.sh" {
      redirect "oznakash@gmail.com";
      stop;
  }
```

### Test 4: Mixed Modes

```
Input:  [
  {"email": "a@x.com", "address": "b@y.com", "keepCopy": true},
  {"email": "c@x.com", "address": "d@y.com", "keepCopy": false}
]

Expected output:
  require ["envelope", "copy"];

  if envelope :is "to" "a@x.com" {
      redirect :copy "b@y.com";
  }

  if envelope :is "to" "c@x.com" {
      redirect "d@y.com";
      stop;
  }
```

### Test 5: Empty List

```
Input:  []
Expected output: "" (empty string)
```

### Test 6: Idempotency

```
Running generate twice with same input produces identical output.
```

### Test 7: Settings API Round-Trip

```
1. POST forwarding config for oz@oznakash.com
2. GET forwarding configs (prefix=forwarding.accounts)
3. Verify returned data matches what was posted
4. Generate script from returned data
5. POST script to sieve.trusted.scripts.auto-forward.contents
6. GET script back
7. Verify script matches expected output
```

---

## Implementation

### What We Build (in this repo)

A standalone script/tool: `scripts/forwarding-manager.sh`

This shell script:
1. Accepts arguments: `add <email> <forward-to> [--no-copy]` or `remove <email>` or `list`
2. Calls the Stalwart Settings API to store/read forwarding configs
3. Regenerates the trusted Sieve script
4. Saves it via the Settings API

Example usage:
```bash
./scripts/forwarding-manager.sh add oz@oznakash.com oznakash@gmail.com
./scripts/forwarding-manager.sh add oz@naka.sh oznakash@gmail.com
./scripts/forwarding-manager.sh remove oz@naka.sh
./scripts/forwarding-manager.sh list
```

### What Would Change in Webadmin (separate repo, future PR)

File: `stalwartlabs/webadmin/src/pages/directory/edit.rs`

- Add "Forward to" text input field after the email field
- Add "Keep local copy" checkbox
- On save: POST forwarding config + regenerate script via Settings API
- On load: GET forwarding config for this account and populate the fields

### API Calls Used

| Action | Method | Endpoint |
|--------|--------|----------|
| Save forwarding config | POST | `/api/manage/settings` |
| Read all forwarding configs | GET | `/api/manage/settings/list?prefix=forwarding.accounts` |
| Read one account's config | GET | `/api/manage/settings/keys?prefixes=forwarding.accounts.<email>` |
| Delete forwarding config | POST | `/api/manage/settings` (type: "clear", prefix) |
| Save generated script | POST | `/api/manage/settings` |
| Set session.data.script | POST | `/api/manage/settings` |

### JSON Payloads

**Save forwarding for an account:**
```json
[
  {
    "type": "clear",
    "prefix": "forwarding.accounts.oz@oznakash.com"
  },
  {
    "type": "insert",
    "prefix": "forwarding.accounts.oz@oznakash.com",
    "values": [
      ["address", "oznakash@gmail.com"],
      ["keep-copy", "true"]
    ],
    "assertEmpty": false
  }
]
```

**Save generated Sieve script:**
```json
[
  {
    "type": "insert",
    "prefix": null,
    "values": [
      ["sieve.trusted.scripts.auto-forward.contents", "require [\"envelope\", \"copy\"];\n\nif envelope :is \"to\" \"oz@oznakash.com\" {\n    redirect :copy \"oznakash@gmail.com\";\n}\n"]
    ],
    "assertEmpty": false
  }
]
```

**Set pipeline trigger (one-time):**
```json
[
  {
    "type": "insert",
    "prefix": null,
    "values": [
      ["session.data.script", "\"auto-forward\""]
    ],
    "assertEmpty": false
  }
]
```

---

## File Structure

```
mail-server/
├── scripts/
│   ├── check-port25.sh              # existing
│   ├── forwarding-manager.sh        # new: CLI tool for managing forwarding
│   └── forwarding-manager-test.sh   # new: test suite
├── ROADMAP.md                        # existing (update Phase 2)
└── FORWARDING-PLAN.md                # this file
```

---

## Sequence: Build Order

1. Write `forwarding-manager-test.sh` (test cases from above)
2. Write `forwarding-manager.sh` (the tool)
3. Run tests, verify all pass
4. Test against live Stalwart instance (API calls)
5. Update ROADMAP.md
6. Commit, push, PR, merge

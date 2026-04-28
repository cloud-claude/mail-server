# Stalwart webadmin fork

Patched build of [stalwartlabs/webadmin](https://github.com/stalwartlabs/webadmin) that adds a **Forwarding** tab to each account's edit page in the Stalwart admin UI.

The Forwarding tab writes a Sieve script that uses Stalwart's alias-based forwarding so the original envelope sender / `Return-Path` is preserved on forwarded mail. This is the alternative to vanilla Sieve `redirect`, which produces a null `Return-Path <>` and tends to land in spam.

## Layout

```
stalwart-webadmin-fork/
├── deploy-webadmin.sh   # Deploys dist/ into the running Stalwart container
├── dist/                # Built WASM + HTML/CSS/JS/icons (the artifact that ships)
└── src-patches/
    └── edit.rs          # Source patch (modified leptos page) — for rebuilding only
```

`dist/` is what actually runs. `src-patches/edit.rs` is kept for documentation and future rebuilds against newer upstream Stalwart versions; it is **not** consumed by the deploy script.

## Deploy

On the VPS, with the Stalwart container running:

```sh
git pull
./stalwart-webadmin-fork/deploy-webadmin.sh
```

The script:
1. Finds the running container via `docker ps | grep -i stalwart` (first match).
2. Backs up the container's existing `/opt/stalwart/webadmin` to `webadmin.bak`.
3. Copies the contents of `dist/` into `/opt/stalwart/webadmin/`.

No restart is needed — the webadmin is static assets served by Stalwart, so a browser refresh picks up the new build.

## Rollback

The deploy script prints the rollback command on completion. It is:

```sh
docker exec <container> sh -c 'rm -rf /opt/stalwart/webadmin && mv /opt/stalwart/webadmin.bak /opt/stalwart/webadmin'
```

## Verify

After deploying, in the admin UI:
1. Open any account → edit → confirm a **Forwarding** tab is present alongside the existing tabs.
2. Set a forwarding target (e.g. a personal Gmail), save.
3. Send a test message to the account; confirm it's forwarded and the `Return-Path` on the delivered mail matches the original sender (not `<>`).

## Rebuilding from source

The fork's only functional change against upstream is in `crates/webadmin/src/pages/directory/edit.rs` — the file in `src-patches/edit.rs` is the patched version. To rebuild:

1. Clone upstream: `git clone https://github.com/stalwartlabs/webadmin && cd webadmin`.
2. Replace `crates/webadmin/src/pages/directory/edit.rs` with `src-patches/edit.rs` from this repo.
3. Build with Trunk: `trunk build --release` (the upstream README has the toolchain prerequisites — Rust + `wasm32-unknown-unknown` target + `trunk`).
4. Copy the resulting `dist/` over `stalwart-webadmin-fork/dist/` here, commit, deploy.

If upstream's `edit.rs` has diverged significantly, the patch will need to be re-applied by hand — there's no automated rebase. The Forwarding additions are localized: a tab definition, a couple of `create_rw_signal` state pairs, the tab body, and `parse_forwarding_target` / `update_forwarding_script` helpers near the bottom.

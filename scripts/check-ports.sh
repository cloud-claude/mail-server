#!/bin/bash
# Check which mail server ports are open (incoming and outgoing)

echo "========================================"
echo "  Mail Server Port Check"
echo "========================================"
echo ""

# --- INCOMING (listening) ---
echo "--- INCOMING (listening on this server) ---"
echo ""

INCOMING_PORTS=(
    "25:SMTP"
    "110:POP3"
    "143:IMAP"
    "443:HTTPS"
    "465:SMTPS"
    "587:Submission"
    "993:IMAPS"
    "995:POP3S"
    "4190:ManageSieve"
    "8080:HTTP Admin"
)

for entry in "${INCOMING_PORTS[@]}"; do
    port="${entry%%:*}"
    name="${entry#*:}"
    if ss -tlnp 2>/dev/null | grep -q ":${port} " ; then
        printf "  %-5s %-15s ✓ LISTENING\n" "$port" "$name"
    else
        printf "  %-5s %-15s ✗ NOT LISTENING\n" "$port" "$name"
    fi
done

echo ""
echo "--- OUTGOING (can connect from this server) ---"
echo ""

OUTGOING_TESTS=(
    "gmail-smtp-in.l.google.com:25:SMTP to Gmail"
    "smtp.gmail.com:465:SMTPS to Gmail"
    "smtp.gmail.com:587:Submission to Gmail"
)

for entry in "${OUTGOING_TESTS[@]}"; do
    host="${entry%%:*}"
    rest="${entry#*:}"
    port="${rest%%:*}"
    name="${rest#*:}"
    result=$(timeout 10 bash -c "echo QUIT | nc -w 5 $host $port" 2>&1)
    if [ $? -eq 0 ]; then
        printf "  %-5s %-25s ✓ OPEN\n" "$port" "$name"
    else
        printf "  %-5s %-25s ✗ BLOCKED/TIMEOUT\n" "$port" "$name"
    fi
done

echo ""
echo "Done."

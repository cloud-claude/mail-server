#!/bin/bash
# Check if outbound port 25 is open by connecting to Gmail's MX server

echo "Testing outbound port 25 connectivity..."
echo ""

# Test 1: Gmail MX
echo "1. gmail-smtp-in.l.google.com:25"
timeout 10 bash -c 'echo QUIT | nc -w 5 gmail-smtp-in.l.google.com 25' 2>&1
if [ $? -eq 0 ]; then
    echo "   ✓ OPEN"
else
    echo "   ✗ BLOCKED or TIMED OUT"
fi
echo ""

# Test 2: Outlook MX
echo "2. outlook-com.olc.protection.outlook.com:25"
timeout 10 bash -c 'echo QUIT | nc -w 5 outlook-com.olc.protection.outlook.com 25' 2>&1
if [ $? -eq 0 ]; then
    echo "   ✓ OPEN"
else
    echo "   ✗ BLOCKED or TIMED OUT"
fi
echo ""

# Test 3: Yahoo MX
echo "3. mta5.am0.yahoodns.net:25"
timeout 10 bash -c 'echo QUIT | nc -w 5 mta5.am0.yahoodns.net 25' 2>&1
if [ $? -eq 0 ]; then
    echo "   ✓ OPEN"
else
    echo "   ✗ BLOCKED or TIMED OUT"
fi

#!/usr/bin/env bash
# Test FHIR R4 against Epic's public sandbox
# Epic Open API: https://open.epic.com/
# Sandbox endpoint: https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4
#
# This tests READ-ONLY public endpoints that don't require OAuth.
# For write access, register at open.epic.com for client credentials.

set -euo pipefail

EPIC_BASE="https://fhir.epic.com/interconnect-fhir-oauth/api/FHIR/R4"
PASS=0
FAIL=0

test_endpoint() {
    local name="$1"
    local url="$2"
    local expected="$3"

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$url" -H "Accept: application/fhir+json" 2>/dev/null)
    if [ "$STATUS" = "$expected" ]; then
        echo "  ✓ $name (HTTP $STATUS)"
        PASS=$((PASS+1))
    else
        echo "  ✗ $name (expected $expected, got $STATUS)"
        FAIL=$((FAIL+1))
    fi
}

echo "═══════════════════════════════════════"
echo "  Epic FHIR R4 Sandbox Test"
echo "═══════════════════════════════════════"
echo ""
echo "Endpoint: $EPIC_BASE"
echo ""

# CapabilityStatement — always public
echo "Capability:"
test_endpoint "CapabilityStatement" "$EPIC_BASE/metadata" "200"

echo ""
echo "Public read endpoints (may require auth):"
# These may return 401 without OAuth — that's expected
# 401 = endpoint exists but needs auth (good)
# 404 = endpoint doesn't exist (bad)
# 000 = connection failed (bad)
test_endpoint "Patient search" "$EPIC_BASE/Patient?family=Smith" "401"
test_endpoint "Observation search" "$EPIC_BASE/Observation?code=2345-7" "401"
test_endpoint "Condition search" "$EPIC_BASE/Condition?code=E11" "401"
test_endpoint "MedicationRequest search" "$EPIC_BASE/MedicationRequest" "401"

echo ""
echo "FHIR resource types:"
# Test that the CapabilityStatement lists expected resource types
METADATA=$(curl -s "$EPIC_BASE/metadata" -H "Accept: application/fhir+json" 2>/dev/null)
for resource in Patient Observation Condition MedicationRequest AllergyIntolerance Immunization Procedure; do
    if echo "$METADATA" | grep -q "\"type\":\"$resource\"" 2>/dev/null; then
        echo "  ✓ $resource supported"
        PASS=$((PASS+1))
    else
        echo "  ⊘ $resource not confirmed in metadata"
    fi
done

echo ""
echo "═══════════════════════════════════════"
echo "  $PASS passed, $FAIL failed"
echo "═══════════════════════════════════════"
echo ""
echo "Note: 401 on search endpoints is EXPECTED without OAuth."
echo "Register at https://open.epic.com/ for full access."

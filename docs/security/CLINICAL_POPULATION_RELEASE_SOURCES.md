# Clinical Population Release v1 — External Guidance References

Design references used for v1:

- NIST SP 800-226, *Guidelines for Evaluating Differential Privacy Guarantees* (final, March 2025): differential privacy guarantees depend on exact implementation choices, privacy units/neighbouring datasets, parameters, sensitivity/contribution assumptions, and composition/accounting. V1 therefore does not accept a generic `dp=true` marker.
- CMS cell-size suppression policy: beneficiary-related cells of 1-10 must not be displayed and may not be derivable from percentages/formulas. V1 uses 11 only as a conservative software floor for the fixed static-report path, not as a universal privacy guarantee.

The policy values used by a deployment remain jurisdiction/research-governance decisions and may be stricter than these software minima.

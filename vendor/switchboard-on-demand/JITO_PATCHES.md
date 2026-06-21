# Jito Patch Notes

This vendored copy has local changes that are not present in the upstream
Switchboard On-Demand crate version pinned by the workspace.

## Gateway fallback support

- `Gateway::test_gateway` requires a successful HTTP status before treating a
  gateway as reachable. The upstream implementation accepted any non-empty body,
  so an nginx `503 Service Temporarily Unavailable` page could be selected as a
  healthy gateway.
- `Gateway::fetch_signatures_from_encoded` now calls `error_for_status()` and
  parses JSON through `reqwest`, returning an error instead of panicking on
  non-JSON gateway responses.
- `Gateway::url()` exposes the selected URI so the tip-router CLI can log and
  retry alternate oracle gateway URIs.

These changes support the keeper's Switchboard crank path, where the first
oracle gateway in the queue can be down while other oracle gateways are healthy.

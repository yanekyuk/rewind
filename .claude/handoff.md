---
trigger: "AuthRequired errors from sidecar show as [object Object] in UI instead of redirecting to login screen"
type: fix
branch: fix/auth-required-redirect
base-branch: main
created: 2026-03-31
---

## Related Files
- src/hooks/useManifestList.ts (error handling)
- src/hooks/useAuth.ts (error extraction)
- src/components/VersionSelect.tsx (onAuthRequired prop)
- src/App.tsx (wiring onAuthRequired to handleSignOut)
- src/utils/errors.ts (new shared error utilities)
- src/hooks/useManifestList.test.ts (tests)

## Relevant Docs
- docs/specs/auth-ui.md

## Related Issues
None — no related issues found.

## Scope
Fix already implemented on main (uncommitted). Changes:
1. Added src/utils/errors.ts with isAuthRequiredError() and extractErrorMessage()
2. useManifestList accepts onAuthRequired callback, detects AuthRequired errors, redirects to login
3. VersionSelect passes onAuthRequired prop through
4. App.tsx wires onAuthRequired to handleSignOut
5. useAuth.ts uses shared extractErrorMessage instead of inline logic

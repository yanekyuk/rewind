---
title: "Bun test runner with dependency injection"
type: decision
tags: [testing, bun, frontend, hooks]
created: 2026-03-31
updated: 2026-03-31
---

# Bun Test Runner with Dependency Injection

## Context

The frontend test suite originally used vitest with `vi.mock()` for module-level mocking. When tests ran in the same process, mock state from `vi.mock()` calls in one test file contaminated other test files, causing non-deterministic failures depending on file execution order.

## Decision

Migrate from vitest to bun:test and adopt dependency injection for hook testability.

### Test runner

- Use `bun test` as the test runner instead of vitest
- Configure test setup via `bunfig.toml` with a `test-setup.ts` preload that registers happy-dom and jest-dom matchers
- Split the test command into two phases to isolate module mock scopes:
  - `bun test src/hooks/ src/types/` -- hooks and type tests (no module mocks needed)
  - `bun test src/components/ src/App.test.tsx` -- component tests (use `mock.module()`)

### Hook testability

Hooks that call Tauri IPC (`useAuth`, `useGameList`, `useManifestList`) accept an optional `invoke` parameter with a default of the real `tauriInvoke`:

```typescript
export function useAuth(invoke: InvokeFn = tauriInvoke): UseAuthResult { ... }
```

Tests pass a mock function directly instead of using module-level mocks:

```typescript
const mockInvoke = mock() as any;
const { result } = renderHook(() => useAuth(mockInvoke));
```

### Component testability

Components that need mocked hook return values use `mock.module()` with dynamic imports:

```typescript
mock.module("../hooks/useAuth", () => ({
  useAuth: () => mockUseAuth(),
}));
const { LoginView } = await import("./LoginView");
```

## Consequences

- No cross-file mock contamination: each test file controls its own mock state
- Hooks are independently testable without module mocking infrastructure
- Component tests require dynamic imports after `mock.module()` calls
- vitest is removed from devDependencies, reducing the dependency footprint
- The `act(...)` warning from React testing-library is benign and does not affect test correctness

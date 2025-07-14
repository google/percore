# Changelog

## Unreleased

### Breaking changes

- Changed type from `PerCore<T, C, CORE_COUNT>` to `PerCore<[T; CORE_COUNT], C>`.

### Bugfixes

- `exception_free` will restore the exception mask state even if the function passed to it panics,
  if built with `panic = "unwind"`.

## 0.1.0

Initial release.

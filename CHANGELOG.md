# Changelog

## 0.2.1

### New features

- Made `ExceptionLock` `repr(transparent)`.
- Derive zerocopy traits for `PerCore` and `ExceptionLock`. This is behind the new `zerocopy`
  feature, which is enabled by default.

## 0.2.0

### Breaking changes

- Changed type from `PerCore<T, C, CORE_COUNT>` to `PerCore<[T; CORE_COUNT], C>`.

### New features

- Added support for `PerCore<Box<[T]>, C>`. This lets you determine the number of cores at runtime
  rather than having to have it be a compile-time constant. This is only available with the new
  `alloc` feature.
- `ExceptionLock` and `PerCore` now implement `Default`.

### Bugfixes

- `exception_free` will restore the exception mask state even if the function passed to it panics,
  if built with `panic = "unwind"`.

## 0.1.0

Initial release.

# Changelog

## Unreleased

### Bugfixes

- `exception_free` will restore the exception mask state even if the function passed to it panics,
  if built with `panic = "unwind"`.

## 0.1.0

Initial release.

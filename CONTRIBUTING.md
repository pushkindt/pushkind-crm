# Contributing

## Testing Expectations
- Add unit tests for new service and form logic.
- Use `src/repository/mock.rs` to isolate service tests from Diesel.
- Add integration tests under `tests/` when database access is required.

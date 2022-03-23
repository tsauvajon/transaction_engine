## Design
Built for:
- maintenability
- ease of integration with streams instead of files

## Choices


Completely split business logic (ledger, balance, transactions logic) from "infrastructure" logic (how to read, write, format CSVs)
[TODO: diagram]

Use named types for business values


## Known limitations


## TODO:
- e2e test
- ledger.rs tests

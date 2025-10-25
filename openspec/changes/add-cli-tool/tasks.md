## 1. CLI Framework Setup
- [x] 1.1 Create lexum-cli crate
- [x] 1.2 Add clap for argument parsing
- [x] 1.3 Implement command structure
- [x] 1.4 Add global options (--config, --verbose, --format)
- [x] 1.5 Implement output formatting (JSON, table, pretty)

## 2. Server Commands
- [ ] 2.1 Implement `lexum serve` command
- [ ] 2.2 Add `lexum start` daemon mode
- [ ] 2.3 Add `lexum stop` command
- [ ] 2.4 Implement `lexum status` command
- [ ] 2.5 Add `lexum config validate` command

## 3. Index Commands
- [x] 3.1 Implement `lexum index create`
- [x] 3.2 Add `lexum index list`
- [x] 3.3 Add `lexum index info <index>`
- [x] 3.4 Implement `lexum index delete <index>`
- [x] 3.5 Add `lexum index stats <index>`

## 4. Document Commands
- [x] 4.1 Implement `lexum doc index <index>`
- [x] 4.2 Add `lexum doc get <index> <id>`
- [x] 4.3 Add `lexum doc delete <index> <id>`
- [ ] 4.4 Implement `lexum doc bulk <index> <file>` - Phase 2

## 5. Query Commands
- [x] 5.1 Implement `lexum query <index> <query>`
- [ ] 5.2 Add `lexum lql <query>` command - Phase 3
- [ ] 5.3 Add query from file support
- [ ] 5.4 Implement interactive query mode

## 6. Interactive Mode
- [x] 6.1 Implement REPL with readline
- [ ] 6.2 Add command history
- [ ] 6.3 Add autocomplete
- [ ] 6.4 Implement help system

## 7. Documentation & Testing
- [x] 7.1 Add --help for all commands
- [ ] 7.2 Create usage examples
- [ ] 7.3 Add integration tests
- [ ] 7.4 Write user manual


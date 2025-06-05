# rrr-tui

[![CI](https://github.com/recursive-record-registry/rrr-tui/workflows/CI/badge.svg)](https://github.com/recursive-record-registry/rrr-tui/actions)

A terminal user interface (TUI) browser for the Recursive Record Registry (RRR) data format.

## TODO
### v0.1.0
* Make panes resizeable
    * Top row vertically
    * Tree and Overview panes horizontally
* Implement tree pane
* Implement overview pane
* Implement hexadecimal byte string record search

### Backlog
* Add a record title metadata field to the RRR format
* Horizontal scrolling in metadata pane
* Consider using taffy's `DetailedGridInfo` for drawing the main view panes' edges
* Record opening
    * Disable form elements while a record is being searched for
* Syntax highlighting
* Implement tree persistence

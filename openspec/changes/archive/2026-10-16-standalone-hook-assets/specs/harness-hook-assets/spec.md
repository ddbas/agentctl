## ADDED Requirements

### Requirement: Harness hook files are standalone assets copied on install

Each harness adapter SHALL keep the hook files it installs as standalone files
in an `assets/` directory colocated with the adapter, whose layout mirrors the
install destination. `agentctl install <harness>` SHALL write those files
verbatim to their destinations, and the adapter source SHALL NOT contain the
hook file contents. The assets SHALL be embedded in the built CLI bundle so no
files are read from the source tree at runtime.

#### Scenario: Installed files equal the assets

- **WHEN** `agentctl install pi` or `agentctl install claude` runs
- **THEN** each installed file is byte-identical to its asset file

#### Scenario: Built CLI is self-contained

- **WHEN** the bundled `cli.cjs` runs `agentctl install <harness>` from a
  directory without the source tree
- **THEN** the install succeeds

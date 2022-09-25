# Picus

[![status-badge](https://github-ci.fonona.net/api/badges/windsource/picus/status.svg)](https://github-ci.fonona.net/windsource/picus)

Picus connects to a [Woodpecker CI](https://woodpecker-ci.org) server and
creates an agent in the cloud when there are pending jobs. The
agent will be shutdown when there are no more build jobs for a specific time in
order to reduce cloud costs.
Picus polls the Woodpecker API and starts an agent in the cloud when it is
required. Currently [Hetzner cloud](https://www.hetzner.com/cloud) is supported.

## Usage

Picus provides a container image which can be used with `docker-compose` to
start the service:

```yml
# docker-compose.yml
version: '3'

services:
  picus:
    # Better replace latest with specific version in next line
    image: windsource/picus:latest
    restart: always
    environment:
      - PICUS_WOODPECKER_SERVER=https://woodpecker.example.com
      - PICUS_WOODPECKER_TOKEN=<...>
      - PICUS_AGENT_WOODPECKER_SERVER=woodpecker.example.com:443
      - PICUS_AGENT_WOODPECKER_AGENT_SECRET=<...>
      - PICUS_HCLOUD_TOKEN=<...>
      - PICUS_HCLOUD_SERVER_TYPE=cx21
      - PICUS_HCLOUD_LOCATION=nbg1
      - PICUS_HCLOUD_SSH_KEYS=me@example.com
      - PICUS_HCLOUD_ID=my-woodpecker-instance
```

The following environment variables can or have to be used:

Name | Description | Default
---- | ----------- | -------
`PICUS_WOODPECKER_SERVER` | URL to the Woodpecker host like `https://woodpecker.example.com` | -
`PICUS_WOODPECKER_TOKEN` | Personal token to Woodpecker | -
`PICUS_POLL_INTERVAL` | Interval in which Picus will poll the Woodpecker API `/api/queue/info`.  For format see [parse_duration](https://docs.rs/parse_duration/latest/parse_duration/). | 10s
`PICUS_MAX_IDLE_TIME` | Duration to wait after the last running job before shutting down an agent. For format see [parse_duration](https://docs.rs/parse_duration/latest/parse_duration/). | 30m
`PICUS_AGENT_WOODPECKER_SERVER` | See [Woodpecker doc](https://woodpecker-ci.org/docs/administration/agent-config#woodpecker_server) | -
`PICUS_AGENT_WOODPECKER_AGENT_SECRET` | See [Woodpecker doc](https://woodpecker-ci.org/docs/administration/agent-config#woodpecker_agent_secret) | -
`PICUS_AGENT_WOODPECKER_GRPC_SECURE` | See [Woodpecker doc](https://woodpecker-ci.org/docs/administration/agent-config#woodpecker_grpc_secure) | `true`
`PICUS_HCLOUD_TOKEN` | API token for Hetzner cloud | -
`PICUS_HCLOUD_SERVER_TYPE` | Server type in Hetzner cloud to use for agent | `cx11`
`PICUS_HCLOUD_LOCATION` | Location to start server in Hetzner cloud | `nbg1`
`PICUS_HCLOUD_SSH_KEYS` | List of ssh keys to apply to the server separated by comma | -
`PICUS_HCLOUD_ID` | Unqiue id to identify resources created in Hetzner Cloud for this instance of Picus. Used to separate resources for different Picus installations in Hetzner cloud project. Observe limitation in RFC 1123.| `picus-test`

## Development

Picus is written in Rust. The project provides a VSCode devcontainer which
contains all required tools.

### Build

```shell
cargo build
```

### Run

```console
cargo test
```

In order to run the tests with Hetzner cloud as wll, provide the required
environment variables and run

```console
cargo cargo test -- --ignored
```


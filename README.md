# Nexus commandline client

This tool can help automate operations needed for java artifact management,
especially releases and their staging operations when publishing to Maven Central repository.


## Commandline interface

Basic commands:

```
Sonatype Nexus Unofficial Client

Usage: nexus <COMMAND>

Commands:
  download  Download repository - entire or a subtree
  upload    Upload local dir to a repository
  ls        List a directory in a remote repository
  rm        Remove a path on remote repo (file of directory with its contents)
  staging   Manage staging repositories. Only for Nexus instances with "staging plugin" configured
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Staging subcommands:

```
Manage staging repositories. Only for Nexus instances with "staging plugin" configured

Usage: nexus staging <COMMAND>

Commands:
  profiles  Show available staging profiles
  profile   Show one staging profile
  repos     Show all current staging repositories
  repo      Show one staging repository
  activity  Retrieve current activity status on a staging repository
  start     Create a new staging repository
  finish    Finish (close) staging repository, exposing it to others for consuming
  promote   Promote (release) staging repository into the target repository (typically `releases`)
  drop      Drop staging repository
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

Staging is used to create temporary repository, which can be later checked and promoted to a target repository.

Most typical use-case for this is when you want to publish into [Maven Central Repository](https://mvnrepository.com/).

## Configuration

None (yet).

## Environment variables

### `NEXUS_URL`

Defaults to `https://oss.sonatype.org`.

Identifies the nexus instance (server) that we are trying to connect to.

### `NEXUS_AUTH`

Authentication information for the Nexus server, in format `<user>:<password>`.

### `NEXUS_STAGING_PROFILE`

When working with staging functionality of Nexus, especially the OSS instance gate-keeping entry to the Maven Central,
you need to provide a "staging profile" used to create a staging repository.

It's a bit tedious to put it in every CLI command, so you can just store it in this variable.

## Authentication

Authentication can be done:
- using environment variable `NEXUS_AUTH` with `<user>:<password>` in it
- using `~/.netrc` entry (experimental!)

Other ways of storing auth credentials are being considered. Suggestions are welcome.

## Useful references

* [Uploading to a Staging Repository via REST API](https://support.sonatype.com/hc/en-us/articles/213465868-Uploading-to-a-Staging-Repository-via-REST-API)
* [Nexus Staging Plugin REST API](https://oss.sonatype.org/nexus-staging-plugin/default/docs/index.html)
* [OSS Sonatype upload script](https://github.com/pkozelka/libtorch-bundle/blob/main/upload.sh)

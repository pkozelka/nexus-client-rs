# Nexus commandline client

The goal of this tool is to help automate operations needed for artifact management,
especially releases and their staging operations.


## Commands

Note: this is just a draft proposal; implementation may be different, the docs gets occasionally updated

- `stagingrepo-ls` - list staging repositories
- `stagingrepo-show` - show details of the repo
- `stagingrepo-close` - close the staged repo
- `stagingrepo-release` - release closed repo
- `stagingrepo-drop` - drop a repository (cancelling release)

## Authentication

Authentication can be done:
- using environment variable `NEXUS_CLIENT_AUTH` with `<user>:<password>` in it, and `NEXUS_URL` to indicate the instance of Nexus
- using `~/.netrc` entry, with host `oss.sonatype.org`

## Useful references

* [Uploading to a Staging Repository via REST API](https://support.sonatype.com/hc/en-us/articles/213465868-Uploading-to-a-Staging-Repository-via-REST-API)
* [Nexus Staging Plugin REST API](https://oss.sonatype.org/nexus-staging-plugin/default/docs/index.html)
* [OSS Sonatype upload script](https://github.com/pkozelka/libtorch-bundle/blob/main/upload.sh)

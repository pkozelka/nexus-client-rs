# Trying the Staging API with CURL

## Start staged repo

```shell
$ curl -u "$NEXUS_AUTH" -H "accept: application/json" -H "content-type: application/json" -v https://oss.sonatype.org/service/local/staging/profiles/3ce9ff9ad792da/start \
  -d '{"data": {"description":"xyz"}}'
# 1st time:  returns HTTP/2 201
{"data":{"stagedRepositoryId":"aih2o-2237","description":"test"}}
# 2nd time: returns HTTP/2 201
{"data":{"stagedRepositoryId":"aih2o-2238","description":"test"}}
```

With invalid profile id, returns `HTTP/2 404`:
```json
{"errors":[{"id":"*","msg":"Cannot create Staging Repository, profile with id '12345' does not exist."}]}
```

## Drop staged repo

```shell
$ curl -u "$NEXUS_AUTH" -H "accept: application/json" -H "content-type: application/json" -v https://oss.sonatype.org/service/local/staging/profiles/3ce9ff9ad792da/drop \
  -d '{"data": {"stagedRepositoryId":"aih2o-2237"}}'
# 1st time: returns HTTP/2 201, content-length: 0
# 2nd time: returns HTTP/2 500
{"errors":[{"id":"*","msg":"Unhandled: Missing staging repository: aih2o-2237"}]}
# with invalid repo id: HTTP/2 500
{"errors":[{"id":"*","msg":"Unhandled: Missing staging repository: aaa"}]}
# with missing stagedRepositoryId: HTTP/2 500
{"errors":[{"id":"*","msg":"Unhandled: null"}]}
# with invalid json - using text "HELLO" instead:  HTTP/2 500 and HTML content
```

## Upload file

```shell
# store to `/a/b/c/d/Cargo.toml`
$ curl -u "$NEXUS_AUTH" -H "accept: application/json" -H "content-type: application/json" -v \
  https://oss.sonatype.org/service/local/staging/deployByRepositoryId/aih2o-2230/a/b/c/d/Cargo.toml \
  --upload-file Cargo.toml
# returns HTTP/2 201, content-length: 0
# 2nd time, the same
# then store something to `/a/b/c/d`
# returns HTTP/2 500, with html content
```

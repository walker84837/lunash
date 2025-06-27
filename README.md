# Lunash

A blazing-fast Bash replacement powered by Lua scripting, featuring
- built-in HTTP requests
- regex
- string/path utilities
- PATH-style script discovery
written in Rust.

## Running a script

To avoid confusing your script for a regular Lua file, each Lunash script is named in the format of `your-script-name.lunash.lua`.

The runner checks in:

1. The current directory
2. Lunash's [local data directory](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.data_local_dir).
3. `LUA_SCRIPT_PATH` environment variable: a colon-separated list of directories, like `PATH`

## Examples

### HTTP request

`http-request.lunash.lua`:
```lua
local body = http:get("http://httpbin.org/get")
print(body)
```

Running the Lua script:
```
$ cargo r -q -- run http-request
{
  "args": {},
  "headers": {
    "Accept": "*/*",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-685ded21-28063cf33278b5856918128e"
  },
  "origin": "39.31.176.66",
  "url": "http://httpbin.org/get"
}
```

*NOTE*: I changed the IP address and generated another one for demonstration purposes.

## Lua API

### `fs` module

- `fs:dirname()`: gets the current directory's parent directory
  - `fs:dirname(string path)`: returns the parent directory of the path
- `fs:readlink(string path)`: reads a symbolic link and returns the path it points to
- `fs:basename(string path)`: strips the specified path from suffixes and directories

### `http` module

- `http:get(string url)`: returns the response body
- `http:post(string url, string body)`: returns the response body

### `stringx` module

- `stringx:split(string str, string delimiter)`: splits a string into an array of lines given a delimiter and returns the array
- `stringx:trim(string str)`: trims whitespace from the beginning and end of a string

### `regex` module

- Initializer: `regex(string expr)`: creates a regex object
  - `regex:is_match(string text)`: returns whether the regex matches the text
  - `regex:find(string text)`: returns a table containing the captured groups from the text

## Roadmap

- [ ] Add overloads for HTTP requests (or an optional constructor?) for specifying things such as API keys, etc.
- [ ] Implement other HTTP methods
  - HEAD
  - PUT
  - DELETE
  - PATCH
  - [Other](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Methods) methods...?


## License

Lunash is licensed under the [MIT](LICENSE) license.

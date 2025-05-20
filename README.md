# localize-cli

`localize-cli` is a straightforward command-line tool designed to translate **JSON localization files** from English into multiple target languages using a translation API.

## Features

- **Recursively scans** a source directory for JSON files.
- **Flattens nested JSON keys** for efficient translation.
- **Translates all unique text values concurrently** with configurable concurrency limits.
- **Rebuilds translated JSON files** while preserving their original structure.
- **Saves translated files** into language-specific output folders.

---

## Installation

### From crates.io

```
cargo install localize-cli
```

### From source

```
git clone [https://github.com/rendivs925/localize-cli.git](https://github.com/rendivs925/localize-cli.git)
cd localize-cli
cargo build --release
```

The binary will be located at target/release/localize-cli.

## Usage

```
localize-cli [OPTIONS]
Options
-s, --source <SOURCE>: Source directory containing English JSON files. Default: locales/en
-o, --output <OUTPUT>: Output directory for translated files. Default: locales
-l, --langs <LANGS>: Comma-separated list of target languages (ISO codes). Default: de,id,ja
-c, --concurrency <CONCURRENCY>: Maximum number of concurrent translation requests. Default: 10
-u, --url <URL>: Translation API URL endpoint. Default: http://localhost:5000/translate
-h, --help: Print help information.
-V, --version: Print version information.
```

## Example

Translate all JSON files in locales/en to German (de) and Japanese (ja), saving outputs in the locales directory:

```
localize-cli --langs de,ja
```

Specify a custom translation API URL and concurrency:

```
localize-cli --url https://libretranslate.com/translate --token YOUR_API_KEY
```

Or skip the token (public/local endpoint):

```
localize-cli --url http://localhost:5000/translate
```

## JSON File Format

The tool expects nested JSON files with string values, for example:

```
{
  "greeting": {
    "morning": "Good morning",
    "evening": "Good evening"
  },
  "farewell": "Goodbye"
}
```

## License

MIT License

## Contributing

Contributions and issues are welcome. Feel free to open pull requests or report bugs.

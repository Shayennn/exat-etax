# EXAT E-Tax Invoice Downloader

This is Rust software to download e-tax invoices from the [EXAT](https://www.exat.co.th/) website.

## Usage

```txt
USAGE:
    exat-etax [FLAGS] [OPTIONS] <taxID> [filename]

FLAGS:
    -h, --help           Prints help information
        --no-download    Prevent downloading ZIP file
    -V, --version        Prints version information

OPTIONS:
    -S, --since <since>    Start date of the search (default: today)
    -U, --until <until>    End date of the search (default: today)

ARGS:
    <taxID>       Tax identification number
    <filename>    Custom filename for the downloaded ZIP (optional)
```

## License

This software is licensed under the MIT license. See [LICENSE](LICENSE) for details.

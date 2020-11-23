# donop

A crawler which focused on collecting an information.

It uses an aproach of checking all links on pages.

Curretly it uses webdriver as a backend, which may be a little slower then
just using curl but.
It brings a way to run a custom JS to collect the data.
And a list of other benefits.

*THE PROJECT HAS A WIP STATUS*
*WE ARE AWARE THAT SOME COMMANDS ARE NOT IMPLEMENTED*

## Get started

By default currently `donop` suggests to use [`xenon`](https://github.com/stevepryde/xenon) for multiplexing webdriver connections.

So take a look at it.

After xenon was started you can run a crawler like that.

This command starts 10 workers which will use `bss_check_v2.js` to collect data. Provides a filter via `-f` option which will run a crawl only on this host. And the url from which the crawler will get started its process.

Be sure that xenon configured with at least the same amount of workers as `donop` is.

```
doonop -c bss_check_v2.js -j 10 -f="host-name=https://bsscommerce.com" https://bsscommerce.com/
```

## Flags

```
doonop 1.0
Maxim Zhiburt <zhiburt@gmail.com>

USAGE:
    doonop [OPTIONS] [--] [urls]...

ARGS:
    <urls>...    A site urls from which the process of checking will be started

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --check-file <check-file>
            A path to a Javascript file which considered to return a JSON if the value is different
            from `null` it will be saved and present in the output. By default it saves a url of a
            page

    -f, --filters <filters>...
            Filters which help to cover limitations of regex. Curently there's only 1 filter host-
            name *host-name make sure that only urls within one host-name will be checked. The
            syntax of filters is filter_name=value

    -i, --ignore-list <ignore-list>...
            A list of regex which determines which url paths may be ingored. Usefull for reducing a
            pool of urls which is up to be checked. If any of the regex returns true the url
            considered to be missed. It uses [`regex`](https://docs.rs/regex/1.3.9/regex/) so such
            features as lookahead and negative lookahead are not available. Mostly in regard of
            ?perfomance? (If there will be any demand Might its reasonable to switch to `fancy-
            regex`, but now there's a filter for base url check which covers todays needs)

    -j <count-searchers>
            An amount of searchers which will be spawned

    -l, --limit <limit>
            Limit of found artifacts

    -p, --page-load-timeout <page-load-timeout>
            A page load timeout after crossing which the searcher will skip the URL. Value is
            supposed to be in milliseconds

    -s, --seed-file <seed-file>
            A path to file which used to seed a url pool. A file must denote the following format
            `url per line`
```
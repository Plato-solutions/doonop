# donop

A crawler which focused on collecting an information.

It uses an aproach of checking all links on pages.

Curretly it uses webdriver as a backend, which may be a little slower then
just using curl but.
It brings a way to run a custom JS to collect the data.
And a list of other benefits.

*THE PROJECT HAS A WIP STATUS*

## Get started

By default currently `donop` suggests to use [`xenon`](https://github.com/stevepryde/xenon) for multiplexing webdriver connections.

So take a look at it.

After xenon was started you can run a crawler like that.

This command starts 10 workers which will use `bss_check_v2.js` to collect data. Provides a filter via `-f` option which will run a crawl only on this host. And the url from which the crawler will get started its process.

Be sure that xenon configured with at least the same amount of workers as `donop` is.

```
doonop -c bss_check_v2.js -j 10 -f="host-name=https://bsscommerce.com" https://bsscommerce.com/
```

## Usage

```
doonop 1.0

Maxim Zhiburt <zhiburt@gmail.com>

USAGE:
    doonop [FLAGS] [OPTIONS] [--] [urls]...

ARGS:
    <urls>...    A site urls from which the process of checking will be started

FLAGS:
    -h, --help              Prints help information
        --use_robots_txt    An option to turn off or turn on a robots.txt check
    -V, --version           Prints version information

OPTIONS:
    -b, --browser <browser>
            A webdriver type you're suppose to run it against. The expected options are: - firefox -
            chrome [default: firefox]

    -c, --check-file <check-file>
            A path to a Javascript or Side file. Javascript code must return a JSON if the value is
            different from `null` it will be saved and present in the output. By default it saves a
            url of a page

        --check-file-format <check-file-format>
            A format of a check file

    -f, --filter <filter>...
            Filters can be used to restrict crawling process by exact rules. For example by `domain`
            Example: `-f "domain=google.com"`

    -i, --ignore <ignore>...
            A list of regex which determines which url paths may be ingored. Usefull for reducing a
            pool of urls which is up to be checked. If any of the regex matches a url, it is
            considered to be ignored

    -j <count-searchers>
            An amount of searchers which will be spawned

    -l, --limit <limit>
            Limit of found artifacts

    -p, --page-load-timeout <page-load-timeout>
            A page load timeout after crossing which the searcher will skip the URL. Value is
            supposed to be in milliseconds

        --proxy <proxy>
            Proxy setting. An example of format is
            "sock;address=https://example.net;version=5;password=123;username=qwe". Available types
            are "sock", "http", "auto-config", "auto-detect", "direct", "system"

        --retry-count <retry-count>
            An amount of retries is allowed for a url [default: 3]

        --retry-policy <retry-policy>
            A policy for a retry in case of network/timeout issue. The expected options are: - no,
            no retries - first, prioritize urls for retry - last, prioritize new urls over ones
            which might be retried [default: first]

        --retry_threshold <retry-threshold-milis>
            A threshold value im milliseconds after which a retry might happen [default: 10000]

        --robot <robot-name>
            A robot name which will be used for matching in robot.txt file if it exists [default:
            DoonopRobot]

    -s, --seed-file <seed-file>
            A path to file which used to seed a url pool. A file must denote the following format
            `url per line`

    -w, --webdriver-url <webdriver-url>
            A webdriver address [default: http://localhost:4444]
```
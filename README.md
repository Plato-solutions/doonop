# doonop

Doonop is a crawler and scrapper on behalf.
Focused on collecting arbitrary information from an entire site or its part. 

The goal behind `doonop` is to be a general purpouse crawling/scraper,
where you don't need to write any specific code and do it pretty fast.

To collect data `doonop` uses a `.js` or `.side` file.

## Design

It uses Webdriver to traverse the site and collect information.
It spawns `N` engines to do things in parallel.

Becouse we spawn `N` engings you have to run some of webdriver multiplexors with at least `N` connections.
For example you can use any webdriver hub for example:

- https://github.com/stevepryde/xenon
- https://www.selenium.dev/documentation/en/grid/
- https://github.com/aerokube/selenoid

The binary was tested only with [stevepryde/xenon](https://github.com/stevepryde/xenon).

## Requirements

As have been already mentioned you need any implementation of a Webdriver (`chromedriver` or `geckodriver`).
But more importantly you need a Webdriver Hub.

You can peck any from your faivorites.

Personally in developing cycle I am using https://github.com/stevepryde/xenon.
You can run it in a docker as well if you like it.

Then install `doonop` itself.
You can install it from sources https://github.com/Plato-solutions/doonop.

```
git clone https://github.com/Plato-solutions/doonop 
```

## Get started

In this example we're going to crawl 5 pages on wiki and collect it's urls with a timestamp when they were obtained. 

Let's prepare a `.js` file first. The `.js` file will collect a page url and a timestamp.

```js
return {
    "url": window.location.href,
    "time": new Date().toLocaleString()
};
```

Then start Webdriver Hub of your choise.

After xenon was started you can run `doonop` that.
Assuming you're running at least 1 instance of `geckodriver`

```bash
doonop --browser=firefox --limit 5 --check-file='./tests/resources/readme.js' --filter=domain=www.en.wikipedia.org https://en.wikipedia.org/wiki/Main_Page/
```

You can find an explanation for each used option using a `--help` argument. 

It will result in the following output.

```json
{"time":"7/20/2021, 9:19:17 PM","url":"https://en.wikipedia.org/wiki/Main_Page"}
{"time":"7/20/2021, 9:19:18 PM","url":"https://en.wikipedia.org/wiki/Wikipedia:General_disclaimer"}
{"time":"7/20/2021, 9:19:20 PM","url":"https://en.wikipedia.org/w/index.php?title=Wikipedia:General_disclaimer&printable=yes"}
{"time":"7/20/2021, 9:19:22 PM","url":"https://en.wikipedia.org/w/index.php?title=Special:UserLogin&returnto=Wikipedia%3AGeneral+disclaimer&returntoquery=printable%3Dyes"}
{"time":"7/20/2021, 9:19:23 PM","url":"https://en.wikipedia.org/wiki/Special:UserLogin"}
```

## Help

To get more information about available options you can run help command
`doonop -h`.

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

## `doonop` vs scrapping via scripts

If you're find with writting a `python` script for scrapping and
it works for you that's great.

Though I think that just run a `doonop` command from shell may be as fast as scripting (or even faster).

But to be a candor as `doonop` uses `Webdriver` it may be slower for a small url set.

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

## `doonop` vs scrapping vs Scripts

If you're find with writting a `python` script for scrapping and
it works for you that's great.

Though I think that just run a `doonop` command from shell may be as fast as scripting (or even faster).

But to be a candor as `doonop` uses `Webdriver` it may be slower for a small url set.

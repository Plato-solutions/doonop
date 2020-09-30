#!/usr/bin/bash

# the script require to have an xpath installed
# on arch linux it can be retrived from AUR pakage `perl-xml-xpath`

url=$1
robots="${url}/robots.txt"

# gets a sitemap urls
# todo: I don't know how to split these 2 commands :(
# specifically how to manage new lines
site_maps=$(curl --silent $robots | awk '/Sitemap:/{print $2}')

for map in $site_maps; do
    case $map in *.xml)
        xml=$(curl --silent $map)
        echo $xml | xpath -q -e "/urlset/url/loc/text()"
        ;;
    *)
        echo "site map has not have .xml extension"
        ;;
    esac
done

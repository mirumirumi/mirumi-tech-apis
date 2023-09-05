from __future__ import annotations

import re
import urllib.parse
from constants import *
from opengraph_py3 import OpenGraph


def customize_html(html: str) -> str:
    # remove `\n` after `<br />`
    html = remove_n_after_br(html)

    # add TOC attributes
    html = add_toc_attrs(html)

    # convert `[http(s)://~~~]` -> blog card
    html = convert_to_blogcard(html)

    # convert `:::info/alert/rewrite` -> common box
    html = convert_to_common_box(html)

    # add date into rewrite box
    html = add_date_into_rewritebox(html)

    # add github content domain in `img`
    html = fix_img_src(html)

    # add any classes (`win11_ss` etc...)
    # unimplemented in markdown2: https://github.com/trentm/python-markdown2/issues/299

    return html


def remove_n_after_br(html: str) -> str:
    result: str = html.replace("<br />\n", "<br />")
    return result


def add_toc_attrs(html: str) -> str:
    headings = re.findall("((<h[234])>(.*?)(<\/h[234]>))", html)

    for i, head in enumerate(headings):
        html = re.sub(
            headings[i][0],
            f'{head[1]} id="{urllib.parse.quote(head[2])}" class="toc_item" data-toc-index="{i + 1}">{head[2]}{head[3]}',
            html,
        )

    return html


def convert_to_blogcard(html: str) -> str:
    links = re.findall("(<p.*?>\[((https?://([^\/]+))?(.*?))\](<\/p>)?)", html)  # https://regex101.com/r/qmA70D/1

    for link in links:
        blogcard_tags = """
<a href="##fullpath##" class="blogcard" rel="noopener" target="_top">
    <div class="blogcard">
        <div class="thumbnail ##github##">
            <img src="##image##" alt="##title##" />
        </div>
        <div class="content">
            <div class="title">##title##</div>
            <div class="snippet">##description##</div>
            <div class="footer">
                <div class="favicon">
                    <img src="https://www.google.com/s2/favicons?domain=##domain##" alt="external-site-favicon" />
                </div>
                <div class="domain">##domain##</div>
            </div>
        </div>
    </div>
</a>
"""[
            1:-1
        ]

        fullpath: str = link[1]

        if fullpath.startswith("/"):  # internal link
            domain: str = SITE_DOMAIN
            for_ogp = "https://" + domain + fullpath
            image: str = OpenGraph(for_ogp).get("image", "/assets/no-image.jpg")
            title: str = OpenGraph(for_ogp).get("title", for_ogp)
            description: str = OpenGraph(for_ogp).get("description", "")
        else:
            domain: str = link[3]
            image: str = OpenGraph(fullpath).get("image", "/assets/no-image.jpg")
            title: str = OpenGraph(fullpath).get("title", fullpath)
            description: str = OpenGraph(fullpath).get("description", "")

        blogcard_tags = blogcard_tags.replace("##fullpath##", fullpath)
        blogcard_tags = blogcard_tags.replace("##image##", image)
        blogcard_tags = blogcard_tags.replace("##domain##", domain)
        blogcard_tags = blogcard_tags.replace("##title##", title)
        blogcard_tags = blogcard_tags.replace("##description##", description)

        blogcard_tags = blogcard_tags.replace("##github##", "github" if domain == "github.com" else "")

        html = html.replace(link[0], blogcard_tags)

    return html


def convert_to_common_box(html: str) -> str:
    boxes = re.findall(
        # https://regex101.com/r/YvFeqP/1
        "(<p>:::(info|alert|rewrite\s*\d+\/\d+\/\d+)<\/p>\n*(( *<\/?[a-zA-Z]+\s*.*?>\n*?(.*?(<\/[a-zA-Z]+>)?)?\n)+)<p>:::<\/p>)",
        html,
    )
    replace_to = ""

    for box in boxes:
        if box[1] == "info":
            replace_to = '<div class="box-common box-info">'
        elif box[1] == "alert":
            replace_to = '<div class="box-common box-alert">'
        elif "rewrite" in box[1]:
            replace_to = '<div class="box-common box-rewrite ' + re.sub("rewrite\s*(\d+\/\d+\/\d+)", "\\1", box[1]) + '">'

        html = html.replace(box[0], replace_to + box[2] + "</div>")

    return html


def add_date_into_rewritebox(html: str) -> str:
    boxes = re.findall('(<div class="box-common box-rewrite (\d+\/\d+\/\d+)"><p>)', html)

    for box in boxes:
        html = html.replace(box[0], box[0] + '<span class="rewrite-date">追記 (' + box[1] + ") ：</span>")

    return html


def fix_img_src(html: str) -> str:
    return re.sub(
        '(<p.*?><img.*?src=")((.*?)images\/)(.*?)(".*?\/>\s*\n?.*?<\/p>)',  # https://regex101.com/r/8ckfux/1
        "\\1https://raw.githubusercontent.com/mirumirumi/mirumi-tech-content/main/images/\\4\\5",
        html,
    )

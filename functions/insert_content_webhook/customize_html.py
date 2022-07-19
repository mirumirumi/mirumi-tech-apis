import re
import urllib.parse
from opengraph_py3 import OpenGraph


def customize_html(html: str) -> str:
    # remobe `\n` after `<br />`
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
    return html.replace("<br />\n", "<br />")


def add_toc_attrs(html: str) -> str:
    headings = re.findall("((<h[234])>(.*?)(<\/h[234]>))", html)

    for i, head in enumerate(headings):
        html = re.sub(headings[i][0], f"{head[1]} id=\"{urllib.parse.quote(head[2])}\" class=\"toc_item\" data-toc-index=\"{i + 1}\">{head[2]}{head[3]}", html)

    return html


def convert_to_blogcard(html: str) -> str:
    links = re.findall("(<p.*?>\[(https?://(.*?))\]<\/p>)", html) # https://regex101.com/r/7rZSTQ/1

    for link in links:
        blogcard_tags = """
            <a href="##fullpath##" class="blogcard" rel="noopener" target="_top">
                <div class="blogcard">
                    <div class="thumbnail ##github##">
                        <img src="##image##" alt="##title##" />
                    </div>
                    <div class="content">
                        <div class="title">
                            ##title##
                        </div>
                        <div class="snippet">
                            ##description##
                        </div>
                        <div class="footer">
                            <div class="favicon">
                                <img src="https://www.google.com/s2/favicons?domain=##domain##" alt="external-site-favicon" />
                            </div>
                            <div class="domain">
                                ##domain##
                            </div>
                        </div>
                    </div>
                </div>
            </a>
        """

        fullpath: str = link[1]
        image: str = OpenGraph(link[1])["image"]
        domain: str = re.sub("^(https?:\/\/)?([^\/]+).*$", "\\2", link[2])
        title: str = OpenGraph(link[1])["title"]
        description: str = OpenGraph(link[1])["description"]

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
        "(<p>:::(info|alert|rewrite\s*\d+\/\d+\/\d+)\n*(.*?)(<\/p>\n+((<p>.*?<\/p>\n+)*)\n+<p>)*(.*?)\n*:::<\/p>)",  # https://regex101.com/r/WCIqj7/1
        html
    )
    replace_to = ""

    for box in boxes:
        if box[1] == "info":
            replace_to = "<div class=\"box-common box-info\">"
        elif box[1] == "alert":
            replace_to = "<div class=\"box-common box-alert\">"
        elif "rewrite" in box[1]:
            replace_to = "<div class=\"box-common box-rewrite " + re.sub('rewrite\s*(\d+\/\d+\/\d+)', '\\1', box[1]) + "\">"

        html = html.replace(box[0], replace_to + "<p>" + box[2] + "</p>" + box[4] + "<p>" + box[6] + "</p>" + "</div>")

    return html


def add_date_into_rewritebox(html: str) -> str:
    boxes = re.findall("(<div class=\"box-common box-rewrite (\d+\/\d+\/\d+)\"><p>)", html)

    for box in boxes:
        html = html.replace(box[0], box[0] + "<span class=\"rewrite-date\">追記 (" + box[1] + ") ：</span>")

    return html


def fix_img_src(html: str) -> str:
    return re.sub(
        "(<p.*?><img.*?src=\")((.*?)images\/)(.*?)(\".*?\/>\s*\n?.*?<\/p>)",  # https://regex101.com/r/8ckfux/1
        "\\1https://raw.githubusercontent.com/mirumirumi/mirumi-tech-content/main/images/\\4\\5",
        html
    )

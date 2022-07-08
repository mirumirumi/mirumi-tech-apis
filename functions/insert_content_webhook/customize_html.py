import re


def customize_html(html: str) -> str:

    # add TOC attributes
    html = add_toc_attrs(html)









def add_toc_attrs(html: str) -> str:
    headings = re.findall("((<h[234])>(.*?)(<\/h[234]>))", html)

    for i, head in enumerate(headings):
        html = re.sub(headings[i][0], f"{head[1]} id=\"{urllib.parse.quote(head[2])}\" class=\"toc_item\" data-toc-index=\"{i + 1}\">{head[2]}{head[3]}", html)

    return html


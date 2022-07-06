from __future__ import annotations
from typing import Any, cast, Final, Literal, TypedDict

import os
import re
import json
import boto3
import secret
import base64
import requests
import markdown2
from Post import Post
from proxy_response import *
from datetime import datetime
from zoneinfo import ZoneInfo
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)

JST = ZoneInfo("Asia/Tokyo")


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)
    changed_file_paths = json.loads(event["body"])["files"].split(" ")
    logger.info(changed_file_paths)

    # get contents of changed file

    endpoint_base = "https://api.github.com/repos/mirumirumi/mirumi-tech-content/contents/posts/"
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": secret.GITHUB_PAT,
    }

    changed_posts: list[str] = list()

    for changed_file_path in changed_file_paths:
        if not "posts/" in changed_file_path:
            continue
        if changed_file_path == "posts/template.md":
            continue
        changed_posts.append(re.sub("posts\/(.*?.md)$", "\\1", changed_file_path))  # https://regex101.com/r/ZHOHDe/1

    posts_to_insert :list[Post] = list()

    for file_name in changed_posts:
        res = requests.get(endpoint_base + file_name, headers=headers, timeout=(9.0, 90.0))

        try:
            res.raise_for_status()
        except Exception as e:
            logger.exception(str(res.status_code) + ": " + str(e))
            return s500()
        else:
            body_md = base64.b64decode(json.loads(res.text)["content"].encode()).decode()
            post = Post(slag=file_name, body=body_md)
            posts_to_insert.append(post)

    # markdown to html
    for post in posts_to_insert:
        post.body = markdown2.markdown(post.body)
        post.body = post.body.replace("\n", "")

    # customize HTMLs






    # insert table
        # created_atについてだけ気をつけて！





    return s200()

from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import re
import boto3
import secret
import base64
import requests
import markdown2
from Post import Post
from proxy_response import *
from customize_html import customize_html
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)
    changed_file_paths = event["body"].split(" ")
    logger.info(changed_file_paths)

    endpoint_base = "https://api.github.com/repos/mirumirumi/mirumi-tech-content/contents/posts/"
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": secret.GITHUB_PAT,
    }

    changed_posts: list[str] = list()
    for changed_file_path in changed_file_paths:
        if not "posts/" in changed_file_path:
            continue
        if changed_file_path == "posts/__template.md":
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
            body_md = base64.b64decode(res.json()["content"].encode()).decode()
            post = Post(slag=file_name, body=body_md)
            posts_to_insert.append(post)

    for post in posts_to_insert:
        post.body = markdown2.markdown(post.body, extras={
            "fenced-code-blocks": None,
            "highlightjs-lang": None,
            "code-friendly": None,
            "strike": None,
            "tables": None,
        })

    for post in posts_to_insert:
        post.body = customize_html(post.body)

    for post in posts_to_insert:
        if post.is_already_exist():  # only difference is `created_at` and `updated_at`
            if post.is_same_day_createdAt_and_updatedAt():  # only difference is `updated_at`
                try:
                    post_table.update_item(
                        Key={
                            "slag": post.slag,
                        },
                        UpdateExpression="""set 
                            title = :title,
                            tags = :tags,
                            body = :body,
                            search_title = :search_title,
                            search_tags = :search_tags
                        """,
                        ExpressionAttributeValues={
                            ":title": post.title,
                            ":tags": post.tags,
                            ":body": post.body,
                            ":search_title": post.seach_title,
                            ":search_tags": post.seach_tags,
                        },
                    )
                except Exception as e:
                    logger.exception(e)
                    return s500()
            else:
                try:
                    post_table.update_item(
                        Key={
                            "slag": post.slag,
                        },
                        UpdateExpression="""set 
                            title = :title,
                            updated_at = :updated_at,
                            tags = :tags,
                            body = :body,
                            search_title = :search_title,
                            search_tags = :search_tags
                        """,
                        ExpressionAttributeValues={
                            ":title": post.title,
                            ":updated_at": post.updated_at,
                            ":tags": post.tags,
                            ":body": post.body,
                            ":search_title": post.seach_title,
                            ":search_tags": post.seach_tags,
                        },
                    )
                except Exception as e:
                    logger.exception(e)
                    return s500()
        else:
            try:
                post_table.update_item(
                    Key={
                        "slag": post.slag,
                    },
                    UpdateExpression="""set 
                        title = :title,
                        created_at = :created_at,
                        tags = :tags,
                        body = :body,
                        search_title = :search_title,
                        search_tags = :search_tags
                    """,
                    ExpressionAttributeValues={
                        ":title": post.title,
                        ":created_at": post.created_at,
                        ":tags": post.tags,
                        ":body": post.body,
                        ":search_title": post.seach_title,
                        ":search_tags": post.seach_tags,
                    },
                )
            except Exception as e:
                logger.exception(e)
                return s500()

    return s200()

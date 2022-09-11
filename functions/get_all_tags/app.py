from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import boto3
from proxy_response import *
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)


class TableTagData(TypedDict):
    tags: list[str]
    search_tags: list[str]


class Result(TypedDict):
    tag: str
    search_tag: str


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)

    try:
        res = post_table.scan(ProjectionExpression="tags, search_tags", )
        posts = cast(list[TableTagData], res["Items"])
    except Exception as e:
        logger.exception(e)
        return s500()

    result: list[Result] = list()
    for post in posts:
        for i, tag in enumerate(post["tags"]):
            if not is_exist_tag(result, tag):
                result.append({
                    "tag": tag,
                    "search_tag": post["search_tags"][i],
                })

    result.sort(key=lambda x: x["tag"])

    return s200(result)


def is_exist_tag(result: list[Result], tag: str) -> bool:
    for r in result:
        if tag == r["tag"]:
            return True
    return False

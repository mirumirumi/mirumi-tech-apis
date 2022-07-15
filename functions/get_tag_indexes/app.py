from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import boto3
from constants import *
from proxy_response import *
from boto3.dynamodb.conditions import Attr
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)

ALLOWED_CLIENT_ORIGIN = os.environ["ALLOWED_CLIENT_ORIGIN"]


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)

    if "headers" in event and "origin" in event["headers"]:
        if event["headers"]["origin"] != ALLOWED_CLIENT_ORIGIN:
            return s403()
    elif event["resource"] == "/search-post-from-client":
        return s403()

    page = int(event["queryStringParameters"]["page"])
    tag = event["queryStringParameters"]["tag"]

    result = None
    count = None
    try:
        res = post_table.scan(
            FilterExpression=Attr("tags").contains(tag),
            ProjectionExpression="slag, title, created_at, updated_at",
        )
        result = res["Items"]
        count = len(result)
    except Exception as e:
        logger.exception(e)
        return s500()

    result.sort(key=lambda x: cast(str, x["created_at"]), reverse=True)        

    result = result[(page - 1) * PAGE_ITEMS : page * PAGE_ITEMS]

    return s200({
        "items": result,
        "count": count,
    })

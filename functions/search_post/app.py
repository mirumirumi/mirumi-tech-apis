from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import boto3
from proxy_response import *
from boto3.dynamodb.conditions import Attr, Key
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)
    query: str = event["queryStringParameters"]["query"].lower()

    queries = query.split()
    candidates: list[dict[str, Any]] = list()

    for i, q in enumerate(queries):
        try:
            res = post_table.scan(
                FilterExpression=Attr("slag").contains(query)
                | Attr("search_title").contains(q)
                | Attr("search_tags").contains(q)
                | Attr("search_tags").contains("-".join(queries)),
                ProjectionExpression="slag, title, created_at, updated_at",
            )

            if i == 0:
                candidates = res["Items"]
            else:
                for item in res["Items"]:
                    candidates = filter_only_duplicated(candidates, item)
        except Exception as e:
            logger.exception(e)
            continue  # prioritize returning results over slight variations in search accuracy

    return s200(candidates)


def filter_only_duplicated(candidates: list[dict[str, Any]], item: dict[str, Any]) -> list[dict[str, Any]]:
    filter_to = candidates

    for candidate in filter_to:
        if candidate["slag"] != item["slag"]:
            del candidate

    return filter_to

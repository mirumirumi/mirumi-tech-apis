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


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)

    try:
        res = post_table.scan()
        result = res["Count"]
    except Exception as e:
        logger.exception(e)
        return s500()

    return s200(result)

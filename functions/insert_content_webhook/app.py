from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import json
import boto3
import secret
from proxy_response import *
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

# PowerTools
logger = Logger()

# DynamoDB
POST_TABLE_NAME = os.environ["POST_TABLE_NAME"]
post_table = boto3.resource("dynamodb").Table(POST_TABLE_NAME)


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> ProxyResponse:
    logger.info(event)


    # 更新されたファイル一覧を取得する




    # markdown to html





    # 各HTMLのカスタマイズ






    # テーブルに書き込む






    return s200()

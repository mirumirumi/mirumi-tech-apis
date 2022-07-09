from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import json
import dynamodb


class ProxyResponse(TypedDict):
    statusCode: int
    body: str

def s200(val: object = None) -> ProxyResponse:
    return {
        "statusCode": 200,
        "body": json.dumps(
            val if val is not None else "no response data",
            default=dynamodb.decimal_to_float,
        ),
    }

def s400() -> ProxyResponse:
    return {
        "statusCode": 400,
        "body": "required request data is missing or I/F schema is invalid",
    }

def s403() -> ProxyResponse:
    return {
        "statusCode": 403,
        "body": "required api key is missing or invalid",
    }

def s500(val: object = None) -> ProxyResponse:
    return {
        "statusCode": 500,
        "body": json.dumps(
            val if val is not None else "no response data",
            default=dynamodb.decimal_to_float,
        ),
    }

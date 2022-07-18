from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import os
import hashlib
from aws_lambda_powertools.logging import Logger
from aws_lambda_powertools.utilities.typing import LambdaContext

logger = Logger()

UNHASHED_KEY = os.environ["UNHASHED_KEY"]


@logger.inject_lambda_context
def lambda_handler(event: dict[str, Any], context: LambdaContext) -> dict[str, Any]:
    logger.info(event)

    try:
        if not UNHASHED_KEY == hashlib.sha256(event["authorizationToken"].encode()).hexdigest():
            result("Deny", event)
    except Exception as e:
        print(e)
        return result("Deny", event)

    return result("Allow", event)


def result(effect: str, event: dict[str, Any]) -> dict[str, Any]:
    return {
        "principalId": "*",
        "policyDocument": {
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Action": "execute-api:Invoke",
                    "Effect": effect,
                    "Resource": event["methodArn"],
                },
            ],
        },
    }

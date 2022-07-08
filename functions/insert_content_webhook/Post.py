from __future__ import annotations
from typing import Any, cast, Literal, TypedDict

import re
from datetime import datetime
from zoneinfo import ZoneInfo

JST = ZoneInfo("Asia/Tokyo")


class Post:
    def __init__(self, slag: str, body: str) -> None:
        self.slag = slag
        self.title = self.__front_matter_title(body)
        self.tags = self.__front_matter_tags(body)
        self.body = self.__remove_front_matter(body)
        self.created_at = datetime.now(JST).isoformat()
        self.updated_at = datetime.now(JST).isoformat()

    @staticmethod
    def __front_matter_title(body_md: str) -> str:
        lines = body_md.splitlines()
        for line in lines:
            if (line.startswith("title")):
                return re.sub("title\s*:\s*(.*?)$", "\\1", line)  # https://regex101.com/r/4roRGw/1
        raise Exception("front matter of `title` was not found")
   
    @staticmethod
    def __front_matter_tags(body_md: str) -> list[str]:
        lines = body_md.splitlines()
        for line in lines:
            if (line.startswith("tags")):
                return re.sub("tags\s*:\s*\[(.*?)\]$", "\\1", line).replace(" ", "").split(",")  # https://regex101.com/r/5Z0OqH/1
        raise Exception("front matter of `tags` was not found")

    @staticmethod
    def __remove_front_matter(body_md: str) -> str:
        lines = body_md.splitlines()
        is_second_bar = False
        for i, line in enumerate(lines):
            if line == "---" and not is_second_bar:
                is_second_bar = True
                continue
            if line == "---" and is_second_bar:
                j = 1
                try: 
                    while lines[i + j] == "":
                        j += 1
                    return "\n".join(lines[i + j:])
                except IndexError:
                    raise Exception("content body was not found")
        raise Exception("front matter was not found")

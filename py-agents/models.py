from dataclasses import dataclass
from typing import TypedDict, Optional, List
import os


class Link(TypedDict):
    url: str
    tags: list
    description: str
    content: Optional[str]
    title: Optional[str]

def create_link(url: str, tags: List[str], description: str,
                content: Optional[str] = None, title: Optional[str] = None) -> Link:
    return {
        "url": url,
        "tags": tags,
        "description": description,
        "content": content,
        "title": title,
    }



LOCAL_HOST_NAME="host.docker.internal" #localhost

@dataclass
class DBConfig:
    url_llmdb: str
    url_db: str
    db_name: str
    collection_name: str

    @staticmethod
    def from_config() -> "DBConfig":
        url_llmdb = os.getenv("URL_LLMDB", f'http://{LOCAL_HOST_NAME}:7700')
        url_db = os.getenv("URL_DB", f'mongodb://root:example@{LOCAL_HOST_NAME}:27017')
        db_name = os.getenv("DATABASE_NAME", "db")
        collection_name = os.getenv("COLLECTION_NAME", "links")
        db_config = DBConfig(url_llmdb, url_db, db_name, collection_name)
        return db_config

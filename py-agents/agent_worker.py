import sys
import json
import validators
from typing import Optional

from dns.exception import ValidationFailure

import craw
import db
import llmdb
from models import DBConfig, Link

URL_LLMDB="URL_LLMDB"
URL_DB="URL_DB"
DATABASE_NAME="DATABASE_NAME"
COLLECTION_NAME="COLLECTION_NAME"


def is_string_an_url(url_string: str) -> bool:
    result = validators.url(url_string)
    if isinstance(result, ValidationFailure):
        return False
    return result


class download_worker:
    def __init__(self, config: DBConfig):
        url_db = config.url_db
        url_llmdb = config.url_llmdb
        db_name = config.db_name
        collection_name = config.collection_name

        self._client_db = db.DB(uri=url_db, database_name=db_name, collection_name=collection_name)
        self._client_llmdb = llmdb.LLMDB(uri=url_llmdb)

    def download(self, id: str):
        link_info: Optional[Link] = self._client_db.get_link(id)
        if link_info is None:
            print(f"Doesn't exist this id={id} in the db")
            return
        url = link_info['url']
        print(f"Trying to connect uri={url}")
        if not is_string_an_url(url):
            print(f"Not a url string: {url}")
            return


        print(f"Downloading {url}")
        (content, text) = craw.download_link(url)
        if text is None:
            print(f"--> text was None")
            pass

        print(f"Downloaded size={len(text)}")
        link_info['content'] = text
        self._client_db.update_link(id, link_info)
        llmdb_link = llmdb.LLMDBLink(link=link_info, id=id)
        print(f"Indexing in llmdb data={dir(llmdb_link)}")
        self._client_llmdb.edit_link(llmdb_link)


def main(db_config: DBConfig):
    print("Initializing...")
    print(f"db_config={db_config}")
    if len(sys.argv) < 2:
        print("Usage: agent_worker.py <task_json>")
        return

    task = json.loads(sys.argv[1])
    print(f"[Agent] Processing task: {task}")
    if 'id' not in task:
        print("Agent has to include some id")

    worker = download_worker(db_config)
    worker.download(task['id'])
    print(f"[Agent] Task {task['id']} finished")

if __name__ == "__main__":
    db_config = DBConfig.from_config()
    main(db_config)


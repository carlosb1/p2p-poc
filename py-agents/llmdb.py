import os
from typing import Optional, TypedDict
from uuid import uuid4

import meilisearch

from models import Link, create_link


class LLMDBLink(TypedDict):
    link: Link
    id: str   # extra field

def new_llmdb_link(link: Link) -> LLMDBLink:
    return LLMDBLink(link=link, id=str(uuid4()))

MODEL = "links"
MASTER_KEY = "MASTER_KEY"

class LLMDB:
    def __init__(self, uri: str):
        self._client = meilisearch.Client(uri, MASTER_KEY)
        self._index = self._client.index(MODEL)

    def _wait(self, task):
        task_uid = task.task_uid
        self._client.wait_for_task(task_uid)

    def insert_link(self, link: LLMDBLink, wait=False):
        task = self._index.add_documents([link])
        if wait:
            self._wait(task=task)

    def query(self, query: str, limit=20) -> dict[str, LLMDBLink]:
        res = self._index.search(query, opt_params={"limit": limit})
        return res['hits']
    def query_highlight(self, query: str, limit=20) -> dict[str, LLMDBLink]:
        params = {
            'attributesToHighlight': ['*'],
        }
        res =  self._index.search(query, params=params)
        return res['hits']

    def edit_link(self, link: LLMDBLink, wait=False):
        task = self._index.update_documents([link])
        if wait:
            self._wait(task=task)


    def delete_link(self, id: str, wait=False):
        task = self._index.delete_document(id)
        if wait:
            self._wait(task=task)

    def get_links(self):
        return self._index.get_documents()
    def get_link(self, id: str) -> Optional[LLMDBLink]:
        try:
          return self._index.get_document(id)
        except meilisearch.errors.MeilisearchApiError as err:
            return None

def test_links():
    llmdb = LLMDB(uri="http://0.0.0.0:7700")
    link = create_link(uri="http://elpais.com",tags=["tag1", "tab2"], description="descr")
    llmdb_link = new_llmdb_link(link)
    llmdb.insert_link(llmdb_link,wait=True)
    links = llmdb.query("tag1")
    assert len(links) >= 1
    for link in links:
        restore_id = str(link["id"])
        old = llmdb.get_link(restore_id)
        assert old is not None
        old.link["content"] = "content!!"
        toedited = LLMDBLink(link=old.link, id=old.id)
        llmdb.edit_link(toedited, True)
        edited = llmdb.get_link(toedited['id'])
        assert edited.link["content"] == "content!!"
        llmdb.delete_link(edited.id, True)
        new = llmdb.get_link(LLMDBLink(link=edited.link, id=edited.id))
        assert new is None
    links = llmdb.query("tag1")
    assert len(links) == 0


if __name__ == "__main__":
    test_links()




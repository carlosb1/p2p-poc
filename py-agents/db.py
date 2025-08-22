from bson import ObjectId
from pymongo import MongoClient
from typing import Any, Optional, List

from models import Link, create_link


class DB:
    def __init__(self, uri: str, database_name: str, collection_name: str):
        self._client: MongoClient[Link] = MongoClient(uri)
        self._database = self._client[database_name]
        self._collection = self._database[collection_name]


    def insert_link(self, link: Link) -> str:
        res = self._collection.insert_one(link)
        return str(res.inserted_id)
    def replace_link(self, link: Link):
        self._collection.replace_one(link, link)

    def update_link(self,_id: str,  link: Link) -> int:
        result = self._collection.update_one(
            {"_id": ObjectId(_id)},  # Filter by _id
            {"$set": link}  # Fields to update
        )
        return result.modified_count

    def get_links(self) -> list[Link]:
        res = self._collection.find({})
        return list(res)

    def get_link(self,_id: str) -> Optional[Link]:
        res = self._collection.find_one({"_id": ObjectId(_id)})
        if res is not None and "_id" in res:
            del res["_id"]
        return res



def test_links():
    db = DB(uri="mongodb://root:example@localhost:27017", database_name="db", collection_name="links")
    link = create_link(uri="http://elpais.com",tags=["tag1", "tab2"], description="descr")
    id: str = db.insert_link(link)
    links = db.get_links()
    assert len(links) >= 1
    found_link = db.get_link(id)
    found_link["content"] = "content"
    found_link["title"] = "title"
    db.update_link(id, found_link)
    new_link = db.get_link(id)
    assert new_link["content"] == "content"
    assert new_link["title"] == "title"


if __name__ == "__main__":
    test_links()

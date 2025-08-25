from queue import Queue
import threading
import time, hashlib
from typing import Callable
from urllib.parse import urlparse, urlunparse, parse_qsl, urlencode
import feedparser
import requests
from bindings_p2p.bindings_p2p import ClientWrapper
from url_normalize import url_normalize

import agents.tools.p2p

CLOSE_QUEUE_CMD = "#close"
TRACKING_PARAMS = {"utm_source","utm_medium","utm_campaign","utm_term","utm_content","gclid","fbclid","mc_cid","mc_eid"}

def clean_url(u: str) -> str:
    try:
        u = url_normalize(u)
        parts = list(urlparse(u))
        q = [(k, v) for k, v in parse_qsl(parts[4], keep_blank_values=True) if k not in TRACKING_PARAMS]
        parts[4] = urlencode(q, doseq=True)
        if parts[2].endswith('/') and not parts[2] == '/':
            parts[2] = parts[2].rstrip('/')
        return urlunparse(parts)
    except Exception:
        return u

def entry_to_record(entry, feed_url: str) -> dict:
    link = clean_url(entry.get("link") or entry.get("id") or "")
    title = (entry.get("title") or "").strip()
    summary = (entry.get("summary") or "").strip()
    published = entry.get("published") or entry.get("updated") or ""
    source = urlparse(feed_url).netloc
    uid = hashlib.sha256(link.encode("utf-8")).hexdigest()
    return {
        "uid": uid, "title": title, "url": link, "published": published,
        "summary": summary, "source": source, "feed": feed_url, "ts": int(time.time())
    }

def download_feeds(feeds: list) -> list:
    print(f"Downloading feeds...{feeds}")
    parsed_feeds = []
    for url in feeds:
        print(url)
        feed_url = url
        entries = []
        try:
            resp  = requests.get(url)
            print(f"status code: {resp.status_code}")
            parsed = feedparser.parse(resp.content)
            entries = [entry_to_record(e, feed_url) for e in parsed.entries]
        except Exception as e:
            print(e)
        parsed_feeds.append((feeds, entries))
    return parsed_feeds

def dummy_rss_p2p_agent(q: Queue, name: str, p2p_hook_fun: Callable[[str,str], str], topic):
    keep_running = True
    while keep_running:
        item = q.get()
        if item is None:
            q.task_done()
            break
        if isinstance(item, list) and all(isinstance(x, str) for x in item):
            print(f"->{item}")
            results: list((list, list(dict))) = download_feeds(item)
            # p2p logic
            for (feeds, entries) in results:
                for e in entries[0:3]:
                    url_to_analyse = e['url']
                    #mocking!!
                    import uuid
                    url_to_analyse = e['url'] + "_" + str(uuid.uuid4())[:8]
                    print(f"url to analyse: {url_to_analyse}")
                    key = p2p_hook_fun(topic, url_to_analyse)
                    print(f"key for new validation process: {key}")
        elif item == CLOSE_QUEUE_CMD:
            print(f"Closing rss agent")
            keep_running = False

        print(f"{name} got:", item)
        q.task_done()



FEEDS = [
    # Ejemplos (cámbialos por tus medios preferidos)
    "https://news.google.com/rss/search?q=site:elpais.com&hl=es-419&gl=ES&ceid=ES:es",
#    "https://feeds.bbci.co.uk/news/world/rss.xml",
#    "https://www.reddit.com/r/worldnews/.rss",
]

def mock_function(topic: str, url: str) -> str:
    return "mocked_key"

def test_rss_agent():
    q = Queue()
    t = threading.Thread(target=dummy_rss_p2p_agent, args=(q,"AGENT1",mock_function, "topic1", ))
    t.start()
    time.sleep(1)
    q.put(FEEDS)
    print("Waiting for feeds...")
    time.sleep(10)
    q.put(CLOSE_QUEUE_CMD)
    t.join()



from typing import Optional, Tuple, List
from playwright.sync_api import sync_playwright, TimeoutError as PWTimeoutError

def download_link(link: str) -> Tuple[Optional[str], Optional[str]]:
    html = text = None
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            user_agent=("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                        "(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
            viewport={"width": 1366, "height": 2000},
            java_script_enabled=True,
        )
        page = context.new_page()
        try:
            print(f"Downloading from link={link}")
            page.goto(link, wait_until="domcontentloaded", timeout=45000)
            try:
                page.wait_for_load_state("networkidle", timeout=15000)
            except PWTimeoutError:
                pass

            # Aceptar cookies si aparecen (ajusta selectores si hace falta)
            for sel in [
                "button:has-text('Accept')", "button:has-text('Agree')",
                "button:has-text('I agree')", "button:has-text('Allow all')",
                "button:has-text('Aceptar')", "button:has-text('Consentir')",
                "[data-testid='cookie-accept']",
            ]:
                loc = page.locator(sel).first
                try:
                    if loc.count() and loc.is_visible():
                        loc.click(timeout=2000)
                        break
                except:
                    pass

            # Scroll lazy-load (¡envolvemos como función asíncrona!)
            page.evaluate("""async () => {
                const sleep = (ms) => new Promise(r => setTimeout(r, ms));
                const steps = 20;
                const dy = Math.floor(window.innerHeight * 0.8);
                for (let i = 0; i < steps; i++) {
                    window.scrollBy(0, dy);
                    await sleep(250);
                    if ((window.scrollY + window.innerHeight) >= document.body.scrollHeight) break;
                }
            }""")
            try:
                page.wait_for_load_state("networkidle", timeout=8000)
            except PWTimeoutError:
                pass

            # HTML completo
            html = page.content()

            # Texto visible del body (¡función flecha!)
            text = page.evaluate("""() => {
                return document.body ? (document.body.innerText || "") : "";
            }""")
            print(f"Successfully downloaded from {link}")
        except Exception as e:
            print(f"Error downloading {link}: {e}")
            html = text = None
        finally:
            context.close()
            browser.close()
    return html, text





if __name__ == "__main__":
    (html, text) = download_link("https://elpais.com/espana/2025-08-10/abascal-ataca-a-los-obispos-por-jumilla-no-se-si-su-posicion-es-por-los-ingresos-publicos-que-reciben-o-por-los-casos-de-pederastia.html")
    print(text)
import asyncio
import os
from playwright import async_api


BASE_URL = os.environ.get("TESTSPRITE_BASE_URL") or os.environ.get("BASE_URL") or "https://bbrainfuckk.github.io/qorx/"


async def run_test():
    pw = None
    browser = None
    context = None

    try:
        pw = await async_api.async_playwright().start()
        browser = await pw.chromium.launch(
            headless=True,
            args=[
                "--window-size=1280,720",
                "--disable-dev-shm-usage",
                "--ipc=host",
                "--single-process",
            ],
        )
        context = await browser.new_context()
        context.set_default_timeout(10000)
        page = await context.new_page()

        await page.goto(BASE_URL, wait_until="networkidle", timeout=20000)
        credibility_link = page.locator("a[href$='TECHNICAL_CREDIBILITY.html']").first
        assert await credibility_link.count() == 1
        await credibility_link.click(timeout=10000)
        await page.wait_for_load_state("networkidle", timeout=20000)
        body = await page.locator("body").inner_text(timeout=10000)

        assert "small domain-specific language" in body
        assert "not a general-purpose language" in body
        assert "protobuf envelope" in body
        assert "qstk" in body
        assert "not Forth-compatible" in body
        assert "provider invoice savings" in body
    finally:
        if context:
            await context.close()
        if browser:
            await browser.close()
        if pw:
            await pw.stop()


asyncio.run(run_test())

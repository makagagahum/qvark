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
        body = await page.locator("body").inner_text(timeout=10000)

        assert "Qorx Community Edition" in body
        assert "Qorx Local Pro" in body
        assert "1.0.4" in body
        assert "Community boundary" in body
        assert "public CE binary refuses" in body
    finally:
        if context:
            await context.close()
        if browser:
            await browser.close()
        if pw:
            await pw.stop()


asyncio.run(run_test())

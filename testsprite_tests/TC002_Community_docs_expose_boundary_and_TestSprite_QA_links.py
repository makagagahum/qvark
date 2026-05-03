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

        community_link = page.locator("a[href$='COMMUNITY.html']").first
        assert await community_link.count() == 1
        await community_link.click(timeout=10000)
        await page.wait_for_load_state("networkidle", timeout=20000)
        community_body = await page.locator("body").inner_text(timeout=10000)
        assert "Qorx Community Edition" in community_body
        assert "Qorx Local Pro" in community_body
        assert "daemon" in community_body
        assert "integrate" in community_body

        await page.goto(BASE_URL, wait_until="networkidle", timeout=20000)
        testsprite_link = page.locator("a[href$='TESTSPRITE.html']").first
        assert await testsprite_link.count() == 1
        await testsprite_link.click(timeout=10000)
        await page.wait_for_load_state("networkidle", timeout=20000)
        qa_body = await page.locator("body").inner_text(timeout=10000)
        assert "TestSprite QA" in qa_body
        assert "TESTSPRITE_API_KEY" in qa_body
        assert "Community Edition" in qa_body
    finally:
        if context:
            await context.close()
        if browser:
            await browser.close()
        if pw:
            await pw.stop()


asyncio.run(run_test())

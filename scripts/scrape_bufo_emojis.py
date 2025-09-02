#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "httpx",
#     "beautifulsoup4",
#     "rich",
# ]
# ///
"""
Scrape all custom emoji images from bufo.zone and download them to static/emojis.
"""

import asyncio
import re
from pathlib import Path

import httpx
from bs4 import BeautifulSoup
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()


async def fetch_emoji_urls() -> set[str]:
    """Fetch all unique emoji URLs from bufo.zone"""
    console.print("[cyan]Fetching emoji list from bufo.zone...[/cyan]")
    
    async with httpx.AsyncClient() as client:
        response = await client.get("https://bufo.zone")
        response.raise_for_status()
        
    # Parse HTML
    soup = BeautifulSoup(response.text, 'html.parser')
    
    # Find all image URLs from all-the.bufo.zone
    urls = set()
    for img in soup.find_all('img'):
        src = img.get('src', '')
        if 'all-the.bufo.zone' in src:
            urls.add(src)
    
    # Also find URLs in inline styles or other attributes
    pattern = re.compile(r'https://all-the\.bufo\.zone/[^"\'>\s]+\.(png|gif|jpg|jpeg|webp)')
    for match in pattern.finditer(response.text):
        urls.add(match.group(0))
    
    console.print(f"[green]Found {len(urls)} unique emoji images[/green]")
    return urls


async def download_emoji(client: httpx.AsyncClient, url: str, output_dir: Path) -> str:
    """Download a single emoji and return filename"""
    filename = url.split('/')[-1]
    output_path = output_dir / filename
    
    if output_path.exists():
        return filename
    
    response = await client.get(url)
    response.raise_for_status()
    
    output_path.write_bytes(response.content)
    return filename


async def download_all_emojis(urls: set[str], output_dir: Path) -> int:
    """Download all emojis concurrently with rate limiting"""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    downloaded = 0
    skipped = 0
    
    async with httpx.AsyncClient(timeout=30.0) as client:
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            console=console,
        ) as progress:
            task = progress.add_task(f"[cyan]Downloading {len(urls)} emojis...", total=len(urls))
            
            # Download in batches to avoid overwhelming the server
            batch_size = 10
            urls_list = list(urls)
            
            for i in range(0, len(urls_list), batch_size):
                batch = urls_list[i:i+batch_size]
                tasks = [download_emoji(client, url, output_dir) for url in batch]
                results = await asyncio.gather(*tasks, return_exceptions=True)
                
                for url, result in zip(batch, results):
                    if isinstance(result, Exception):
                        console.print(f"[red]Error downloading {url}: {result}[/red]")
                    else:
                        if (output_dir / result).stat().st_size > 0:
                            downloaded += 1
                        else:
                            skipped += 1
                
                progress.update(task, advance=len(batch))
                
                # Small delay between batches
                if i + batch_size < len(urls_list):
                    await asyncio.sleep(0.5)
    
    return downloaded


async def main():
    """Main function"""
    console.print("[bold cyan]Bufo Emoji Scraper[/bold cyan]\n")
    
    # Setup paths
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    output_dir = project_root / "static" / "emojis"
    
    # Fetch emoji URLs
    urls = await fetch_emoji_urls()
    
    if not urls:
        console.print("[red]No emojis found![/red]")
        return
    
    # Download emojis
    downloaded = await download_all_emojis(urls, output_dir)
    
    console.print(f"\n[bold green]✨ Done! Downloaded {downloaded} images to {output_dir}[/bold green]")
    
    # List what we got
    files = list(output_dir.glob("*"))
    console.print(f"[cyan]Total files in directory: {len(files)}[/cyan]")


if __name__ == "__main__":
    asyncio.run(main())
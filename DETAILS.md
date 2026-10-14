## 🔧 Build and install

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (rustup)
- [Node.js](https://nodejs.org/) (v20.19+) and npm
- C build tools (`build-essential` on Linux, Xcode CLI tools on macOS, Visual Studio Build Tools on Windows)

### 1. Build the frontend

```bash
cd FE
npm install
npm run build
```

Copy the generated files into the backend's `static` folder (folder substitution):

```bash
# Linux / macOS
rm -rf ../BE/static && cp -r build ../BE/static

# Windows (PowerShell)
Remove-Item -Recurse -Force ..\BE\static; Copy-Item -Recurse build ..\BE\static
```

> **Note:** Frontend static files are platform-independent. You can build them on any OS and copy them to another.

### 2. Build the backend

```bash
cd BE
cargo build --release
```

The executable is located at `target/release/meigrss` (Linux/macOS) or `target/release/meigrss.exe` (Windows).

### Platform-specific installation

#### Windows

The build produces `meigrss.exe` with the application icon already embedded. Simply run it by double-clicking or from a terminal.

#### macOS

To create a `.app` bundle with an icon:

```bash
cargo install cargo-bundle
cargo bundle --release
```

The bundle is located at `target/release/bundle/osx/MeigRSS.app`. Copy it to `/Applications/`.

To generate the `.icns` file from a 1024x1024 PNG:

```bash
mkdir icon.iconset
sips -z 16 16 icons/icon.png --out icon.iconset/icon_16x16.png
sips -z 32 32 icons/icon.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32 icons/icon.png --out icon.iconset/icon_32x32.png
sips -z 64 64 icons/icon.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128 icons/icon.png --out icon.iconset/icon_128x128.png
sips -z 256 256 icons/icon.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256 icons/icon.png --out icon.iconset/icon_256x256.png
sips -z 512 512 icons/icon.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512 icons/icon.png --out icon.iconset/icon_512x512.png
cp icons/icon.png icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset -o icons/icon.icns
rm -R icon.iconset
```

#### Linux

```bash
cargo build --release
sudo cp target/release/meigrss /usr/local/bin/
sudo cp icons/icon.png /usr/local/share/icons/meigrss.png
sudo cp meigrss.desktop /usr/share/applications/
```

To uninstall:

```bash
sudo rm /usr/local/bin/meigrss
sudo rm /usr/local/share/icons/meigrss.png
sudo rm /usr/share/applications/meigrss.desktop
```

---

## ▶️ Starting the application

Run the compiled executable. The program will:

1. Create a `data/` folder next to the executable, containing the SQLite database and encryption key
2. Start a local web server on port 3000
3. Automatically open your browser at `http://localhost:3000`

To change the port, set the `MEIGRSS_PORT` environment variable:

```bash
# Linux / macOS
MEIGRSS_PORT=8080 ./meigrss

# Windows (PowerShell)
$env:MEIGRSS_PORT=8080; .\meigrss.exe
```

To stop the program, press `Ctrl+C` in the terminal.

---

## ⚙️ Configuration

### Adding RSS feeds

1. Go to **Feed > Feed** in the menu bar
2. Paste the feed URL and click **Test feed**
3. If the test succeeds, select a category and click **Add feed**

You can also import a feed list via **Feed > Import feeds** using a JSON or OPML file.

> **Note:** Duplicate feeds (same URL) are automatically rejected with a clear message. In OPML files web feeds and their values are saved, but newsletter feeds, settings and favorite articles are not saved.

### Categories

Categories are used to organize feeds. Two default categories are present and cannot be deleted or renamed:

- **Uncategorized** - the default category for regular RSS feeds
- **Newsletter** - the fixed category for all newsletter feeds

You can add, rename, and delete other categories from **Categories > Categories**.

> **Note:** Category names that conflict with the default category translations ("Non categorizzati", "Uncategorized", "Not categorized", "Newsletter") are reserved and cannot be used.

### Settings

From **Settings > Settings** you can configure:

- **Retention period:** how long to keep articles in the database (default: 24 hours). Older articles are automatically deleted at each update, except favorites. Displayed in a human-readable format (e.g. "2 days and 3 hours").
- **Highlighted keywords:** words or phrases to visually highlight in article titles, summaries, and full text. Enter them separated by semicolons (e.g. `Rust;AI;cybersecurity`). Matching is whole-word and Unicode-aware.
- **Automatic update:** when enabled, feeds are updated automatically at the configured interval (default: 1 hour). The frontend automatically detects when new articles are available and refreshes the view.

### Searching articles

The search bar in the main toolbar filters articles by title. The search is performed server-side across all articles in the database (not just the current page). Matching uses whole-word boundaries with Unicode support, so searching for "AI" won't match "pain". Articles that match the substring but not the whole-word are dimmed but still visible.

### Sorting and grouping

Four sorting options are available:

- **Newest first / Oldest first:** standard chronological ordering
- **Feed (newest first) / Feed (oldest first):** groups articles by feed name (A-Z), with internal date ordering. A visual separator appears between different feeds.

### Full text download (scraping)

Full text download is disabled by default for each feed. To enable it:

1. Go to **Feed > Feed**
2. Click the **Enable full text download** button for the desired feed

When enabled, the full text is downloaded automatically when you open an article.

Having a full text is what makes these two things possible:
- Viewing the full article within the application
- Generating AI summaries, which take the full text as input

Scraping is not the only way to obtain it: the full text of any article can also be written by hand, as described below. Both features work off the stored text, so they are available on any article that has one, no matter how it got there.

The scraping engine uses Mozilla's Readability algorithm for reliable content extraction, with a fallback to CSS-selector-based extraction.

> **Important:** Before enabling scraping for a feed, make sure you comply with the website's terms of use. The software author is not responsible for any misuse of this feature. Downloading full text is technically equivalent to saving a web page from your browser for personal reading.

### Manual full text editing

The full text of an article can be edited by hand. This is the only way to give a full text to articles the scraper cannot reach: paywalled pages, sites that block automated requests, or feeds where you deliberately keep scraping off.

1. Open (expand) the article
2. Click **Edit full text**
3. Write the text in the box that replaces the article body
4. Click **Save full text**, or **Cancel** to discard the changes

Closing the article while editing discards any unsaved change: reopening it always shows the saved state.

- The button is always available, including on feeds with scraping disabled and on articles that have no text at all
- Saving a non-empty text stops automatic scraping for that article, so what you wrote is never silently overwritten
- Saving an empty text clears the field, and automatic scraping takes over again the next time you open the article, if it is enabled for that feed
- Once an article has a full text, the **Generate AI summary** button becomes available regardless of the feed's scraping setting
- The saved text is limited to 200 KB

### Automatic AI summary

To enable automatic AI summary generation for a feed:

1. First enable **full text download** for that feed
2. Then click the **Always force AI summary** button

The AI summary will be generated automatically when you open an article from that feed. The feed-provided summary is preserved as a fallback - if you later disable the AI summary option, the original summary reappears.

You can also generate an AI summary manually for individual articles by clicking the **Generate AI summary** button inside the expanded article. The manual button only requires the article to have a full text, so it works on feeds with scraping disabled too, as long as the text was downloaded earlier or written by hand. The automatic option above still requires full text download to be enabled for the feed, since there would otherwise be nothing to summarize.

> **Note:** The returned summary is generated by an AI model and may not be totally accurate - if you want to be extremely secure, check full text for the original information, or visit the original page of the article.

### Feed pausing

Feeds can be manually paused from the feed management page. A paused feed is skipped during updates (both manual and automatic).

If a feed does not respond during an update (timeout), it is automatically paused. A popup will notify you of any paused feeds.

> **Note:** Newsletter feeds are always paused and cannot be activated. The pause, scraping, and category controls are hidden for newsletter feeds in the feed management page, since these properties are fixed. For web feeds, scraping is always enabled and cannot be disabled; pause and category are configurable normally.

### New article indicator

Articles downloaded in the latest refresh are highlighted with an orange left border and subtle background color. The highlight disappears once you open (expand) the article. This state is persisted in the database, so it survives page reloads.

### Newsletter import

MeigRSS can import newsletter emails (EML files) as articles. This allows you to archive and read newsletters within the same interface as your RSS feeds.

To import a newsletter:

1. Export the email as an `.eml` file from your email client
2. Go to **Feed > Import newsletter**
3. Keep **Single file** mode selected and choose the EML file - the title, author, and date are extracted automatically
4. Choose an existing newsletter feed or create a new one by entering a name
5. Confirm the import

#### Importing a whole folder

Switch to **Folder** mode to import many newsletters in one go:

1. Select a folder - every `.eml` file inside it is collected, everything else is ignored
2. The number of files found is shown before the upload starts
3. Choose an existing newsletter feed or create a new one
4. Confirm

All files are imported under the same feed in a single operation. Titles are taken from the message subject, falling back to the file name for messages that have none. At the end a summary reports how many newsletters were imported and how many were skipped, split by reason: already in the archive, no HTML content, or not recognized as a valid email.

Limits per upload: 500 files, 5 MB per file, 100 MB in total.

Each newsletter source gets its own feed (e.g. "Giulio Magini" → `newsletter://giulio_magini`). All newsletter feeds are automatically assigned to the **Newsletter** category and have the following fixed properties:

- **Always paused** - newsletter feeds have no URL to fetch, so they are permanently paused and cannot be activated
- **Full text always enabled** - the full HTML content is extracted from the EML file at import time
- **AI summary configurable** - you can enable "Always force AI summary" on newsletter feeds like any other feed

Imported newsletters are automatically marked as favorites to prevent deletion by the retention purge. The original HTML of the newsletter can be viewed in a new browser tab by clicking the **View newsletter** button inside the expanded article.

Newsletter feeds and their articles are included in feed exports and can be re-imported via the standard JSON import. During import, newsletter feeds (URLs starting with `newsletter://`) are automatically recreated in the Newsletter category if they don't already exist.

> **Note:** EML files are uploaded as raw bytes rather than as decoded text, so newsletters whose MIME headers declare an encoding other than UTF-8 keep their accented characters intact.

### Web feed monitoring

MeigRSS can monitor web pages that don't have an RSS feed. Articles are discovered by scanning the page for links matching a date-based URL pattern (`domain/yyyy/mm/dd/slug`, `domain/yyyy/m/d/slug`, `domain/yyyy/mmm/dd/slug`, `domain/yyyymmdd-slug`, `domain/yyyy-mm-dd/slug`, `domain/dd-mm-yyyy/slug`), using a direct GET call from the program or using an external JavaScript rendering service to handle dynamically loaded content.

> **Note:** Please be very careful with this feature, and use it sparingly and only when necessary, excluding sites that use RSS to provide updates. This feature is intended for occasional use (1-2 times a day) and for news sites that handle high traffic but do not support RSS. Please do not use this feature on small RSS hosts.

#### Setting up the rendering API

A web rendering API is optional. Without it, pages are fetched with a direct GET, which works on server-rendered sites. Configure it only if you need to monitor pages that build their article list with JavaScript. Go to **Settings > API Configuration** and scroll to the **Web Rendering API** section:

1. Select a provider from the dropdown
2. Paste the API key
3. Click **Save and test**

Supported providers (all these providers have a specific paid plan, check their sites for more information):

| Provider | Free tier | JS rendering cost
|---|---|---|
| **Scrape.do** | 1,000 credits/month | ×5 credits (~200 pages/month)
| **ScraperAPI** | 1,000 credits/month | ×10 credits (~100 pages/month)
| **ScrapingBee** | No permanent free tier | ×5 credits
| **Scrapfly** | 1,000 credits (one-time) | ×5 credits
| **ZenRows** | No permanent free tier | ×5 credits

The API key is encrypted with AES-256-GCM, same as the AI API key. Free tier availability and pricing should be verified on each provider's website.

> **Note:** The rendering API is separate from the AI API. You can configure one without the other.

#### Adding a web feed

1. Go to **Feed > Feed**
2. Click the **Web page scan** toggle
3. Paste the URL of the page to monitor (e.g. `https://www.news.online/c/science/`)
4. Click **Test scan** - the system will render the page, with API if configured, with GET call otherwise, and search for article links
5. If articles are found, select a category and click **Add web feed**

The scan looks for links whose URL contains a date pattern (see above). Only links pointing to the same domain as the monitored page are captured.

#### How web feeds work

- **During refresh:** web feeds are processed separately from RSS feeds, after RSS and before the retention purge. Each web feed page is rendered via the configured API or GET call, and new article links are extracted and saved.
- **Full text scraping** is always enabled for web feeds (same as newsletters) and cannot be disabled.
- **Enable API scraping** can be configured for this type of feed, switching from normal GET call to render the page to API calls for dynamic pages that are using JS to load the website.
- **Pause and category** can be configured normally from the feed management page.
- **AI summary** can be enabled via the "Always force AI summary" button, like any other feed.
- **Auto-pause:** if the rendering API times out or fails to connect, the web feed is automatically paused (same behavior as RSS feeds).
- **Article dates** are extracted from the URL path. The publication time is set to the moment of download (UTC).
- **Deduplication** works by URL, same as RSS feeds. Articles already in the database are not re-inserted.
- **Title cleanup:** time prefixes and suffixes (e.g. "08:32", "3 ore fa", "2 hours ago", "il y a 3 heures", "vor 2 Stunden", "hace 1 hora", "há 5 dias") are automatically stripped from extracted titles, in all supported UI languages.

Web feeds are identified in the feed management page with a globe icon, distinguishing them from RSS feeds (RSS icon) and newsletter feeds (email icon).

Web feeds are included in feed exports and can be re-imported via the standard JSON and OPML imports, preserving the `is_web_feed` flag and URL pattern.

---

## 🤖 AI API configuration

MeigRSS supports four AI providers for summary generation:

| Provider | Model | Free plan |
|---|---|---|
| **Mistral 500 chars** | Ministral 3 8B | Yes (Free mode) |
| **Mistral 1000 chars** (default) | Ministral 3 14B | Yes (Free mode) |
| OpenAI | gpt-6-luna | No |
| Claude | claude-haiku-5-5 | No |
| Gemini | gemini-3.5-flash-lite | Yes (with limits) |

The summary prompt adapts to the UI language and to the model: Mistral 500 chars generates shorter summaries (3-5 sentences, 500 chars) while other models generate longer ones (3-10 sentences, 1000 chars).

### Creating a free Mistral account

Mistral offers a free plan with limits. Check their site for more information.

1. Go to [console.mistral.ai](https://console.mistral.ai) and create an account
2. In the **Subscription** section, click **Experiment for free**
3. Verify your phone number (one number per account)
4. Go to the **API Keys** section and generate a new key
5. **Copy it immediately** - it is shown only once

### Saving the API key in MeigRSS

1. Go to **Settings > API Configuration**
2. Select the provider (Mistral 500 chars is recommended for summaries)
3. Paste the API key
4. Click **Save and test**

The key is encrypted with AES-256-GCM and stored in the database. It is never exposed in plain text by any endpoint. The currently saved provider and masked key are displayed in the configuration page.

> **Note:** Mistral 500 chars and Mistral 1000 chars use the same API key. Mistral 500 chars is recommended for summaries: it's faster and uses fewer tokens. If articles are long or complex, use Mistral 1000 chars.

### Plan information

The **Plan information** button asks the AI model itself for information about current plan limits. The returned information is generated by the model and may not be accurate - always verify on the official website of the AI provider you are using.

### Deleting the API configuration

To delete the saved API key and start over, use the **Delete API configuration** button at the bottom of the configuration page.

---

## 💾 Import / Export

### Export

**Feed > Export feeds** make the user choose between JSON and OPML file in order to export all data. For JSON exporting the produced file contains all categories, configured feeds, favorite articles, and application settings (retention period, auto-update configuration, highlighted keywords). OPML files exports only categories, feeds and web feeds with their configurations.

### Import

**Feed > Import feeds** lets you upload an exported JSON or OPML file. During import you can:

- **Keep the original category** of each feed (as saved in the file)
- Or **assign all feeds to a specific category**

Categories present in the file but not in the database are created automatically. Feeds with URLs already in the database are skipped (no duplicates). Favorite articles are re-associated with their feeds and immediately flagged as favorites. Application settings are restored from the file if present.

The import result shows separate counts for feeds and favorite articles imported.

JSON imports are accepted up to 256 MB. Exporting has no size limit of its own, so this ceiling only matters when restoring a very large archive of newsletters.

### Delete all feeds

**Feed > Delete all feeds** removes all feeds and their associated articles. Optionally, you can also delete all categories (except the default one).

---

## ⭐ Favorites

Click the star next to an article to add it to favorites. Favorite articles:

- Are never deleted by the automatic purge
- Are included in feed exports
- Can be filtered in the main view (options are: Favorites excluded, All articles, Favorites only)

---

## 🔤 Feed encoding

MeigRSS automatically handles feeds with non-UTF-8 encoding (ISO-8859-1, ISO-8859-15, Windows-1252). This is common with older European feeds served through proxies like FeedBurner. The encoding is detected and converted transparently.

HTML entities in feed titles, authors, and summaries are decoded automatically.

---

## 🗄️ Data structure

Application data is stored in the `data/` folder next to the executable:

| File | Description |
|---|---|
| `meigrss.db` | SQLite database with feeds, articles, settings |
| `.encryption_key` | Encryption key for API keys (generated on first launch) |

> **Warning:** If you delete the `.encryption_key` file, API keys stored in the database become undecipherable. In that case, you will need to re-enter the API key from the interface.

---

## 🌱 Environment variables

| Variable | Description | Default |
|---|---|---|
| `MEIGRSS_PORT` | Web server port | 3000 |
| `MEIGRSS_DB_PATH` | SQLite database path | `./data/meigrss.db` |
| `MEIGRSS_ENCRYPTION_KEY` | Base64 encryption key (alternative to file) | Auto-generated |

---
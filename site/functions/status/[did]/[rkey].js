// CloudFlare Pages Function to handle /status/:did/:rkey routes
// Injects OG meta tags for social media crawlers

const GRAPHQL_ENDPOINT = 'https://zzstoatzz-quickslice-status.fly.dev/graphql';
const SITE_URL = 'https://status.zzstoatzz.io';

// Social media bot user agents
const BOT_USER_AGENTS = [
  'Twitterbot',
  'facebookexternalhit',
  'LinkedInBot',
  'Slackbot',
  'Discordbot',
  'TelegramBot',
  'WhatsApp',
  'Bluesky',
];

function isSocialBot(userAgent) {
  if (!userAgent) return false;
  return BOT_USER_AGENTS.some(bot => userAgent.includes(bot));
}

async function fetchStatus(did, rkey) {
  const response = await fetch(GRAPHQL_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      query: `
        query GetStatus($did: String!, $rkey: String!) {
          ioZzstoatzzStatusRecord(
            first: 1
            where: {
              did: { eq: $did }
              uri: { endsWith: $rkey }
            }
          ) {
            edges { node { uri did actorHandle emoji text createdAt } }
          }
        }
      `,
      variables: { did, rkey: `/${rkey}` }
    })
  });

  const json = await response.json();
  const statuses = json.data?.ioZzstoatzzStatusRecord?.edges?.map(e => e.node) || [];
  return statuses[0] || null;
}

function getEmojiDisplay(emoji) {
  if (emoji && emoji.startsWith('custom:')) {
    return emoji.slice(7).replace(/-/g, ' '); // "bufo-stab" -> "bufo stab"
  }
  return emoji || '';
}

function getOgImageUrl(emoji) {
  // For custom emojis (bufos), use the bufo.zone image directly
  if (emoji && emoji.startsWith('custom:')) {
    const name = emoji.slice(7);
    return `https://all-the.bufo.zone/${name}.png`;
  }
  // For standard emojis, no image (social platforms will use text)
  return null;
}

function generateOgHtml(status, did, rkey, handle) {
  const emojiDisplay = getEmojiDisplay(status.emoji);
  const title = `@${handle}'s status`;
  const description = status.text
    ? `${emojiDisplay} ${status.text}`
    : emojiDisplay;
  const url = `${SITE_URL}/status/${did}/${rkey}`;
  const imageUrl = getOgImageUrl(status.emoji);

  const imageMetaTags = imageUrl ? `
  <meta property="og:image" content="${imageUrl}">
  <meta name="twitter:image" content="${imageUrl}">` : '';

  const twitterCard = imageUrl ? 'summary_large_image' : 'summary';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title} | status</title>

  <!-- Open Graph -->
  <meta property="og:type" content="website">
  <meta property="og:title" content="${title}">
  <meta property="og:description" content="${description}">
  <meta property="og:url" content="${url}">
  <meta property="og:site_name" content="status">${imageMetaTags}

  <!-- Twitter Card -->
  <meta name="twitter:card" content="${twitterCard}">
  <meta name="twitter:title" content="${title}">
  <meta name="twitter:description" content="${description}">

  <!-- Redirect browsers to the actual page -->
  <meta http-equiv="refresh" content="0;url=${url}">
  <link rel="canonical" href="${url}">
</head>
<body>
  <p>Redirecting to <a href="${url}">${url}</a></p>
</body>
</html>`;
}

export async function onRequest(context) {
  const { request, params, next } = context;
  const { did, rkey } = params;

  const userAgent = request.headers.get('user-agent') || '';

  // If not a social bot, pass through to the SPA
  if (!isSocialBot(userAgent)) {
    return next();
  }

  try {
    const status = await fetchStatus(did, rkey);

    if (!status) {
      // Status not found, let the SPA handle it
      return next();
    }

    const handle = status.actorHandle || did.slice(8, 28);
    const html = generateOgHtml(status, did, rkey, handle);

    return new Response(html, {
      headers: {
        'Content-Type': 'text/html;charset=UTF-8',
        'Cache-Control': 'public, max-age=3600', // Cache for 1 hour
      },
    });
  } catch (error) {
    console.error('Error fetching status for OG tags:', error);
    return next();
  }
}

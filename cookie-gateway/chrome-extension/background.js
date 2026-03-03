// background.js - Service Worker for Cookie Bridge extension

const GATEWAY_URL = 'http://localhost:3456';

chrome.runtime.onInstalled.addListener(() => {
  console.log('Cookie Bridge extension installed');
});

// Listen for messages from popup
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'sendCookies') {
    // Get the active tab
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      chrome.cookies.getAll({ url: tabs[0].url }, (cookies) => {
        // Send to gateway
        fetch(`${GATEWAY_URL}/api/cookies`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            domain: new URL(tab.url[0].url).hostname,
            cookies: cookies.map(cookie => ({
              name: cookie.name,
              value: cookie.value,
              domain: cookie.domain,
              path: cookie.path,
              secure: cookie.secure,
              httpOnly: cookie.httpOnly,
              expirationDate: cookie.expirationDate
            }))
          })
        }).then(response => => {
          if (response.ok) {
            sendResponse({ status: 'success' });
          } else {
            sendResponse({ status: 'error', message: response.statusText });
          }
        });
      } catch (error) => {
        console.error('Failed to send cookies:', error);
        sendResponse({ status: 'error', message: error.toString() });
      });
    } else {
      sendResponse({ status: 'error', message: 'Unknown action' });
    }
  });
});

// Listen for tab updates
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tabInfo) {
  if (changeInfo.status === 'complete' && tabInfo.url) {
    // Update popup with new URL
    chrome.action.setBadgeText({ text: 'Refreshing...' });
    chrome.action.openPopup();
  }
});

// Listen for messages from popup
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'refresh') {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      chrome.cookies.getAll({ url: tabs[0].url }, (cookies) => {
        const gatewayUrl = 'http://localhost:3456';
        const domain = new URL(tabId .url).hostname;

        // Check whitelist
        const isWhitelisted = domain.endsWith('.company.com') ||
                     domain === 'api.company.com' ||
                     domain === 'internal.company.net');

        if (!isWhitelisted) {
          sendResponse({ status: 'error', message: 'Domain not whitelisted' });
          return;
        }

        // Check opt-in status
        fetch(`${gatewayUrl}/api/cookies?domain=${domain}`)
          .then(response => response.json())
          .then(data => {
            if (data.opted_in) {
              chrome.storage.local.get(['cookie_opt_in'], (result) => {
                if (result.opted_in) {
                  chrome.tabs.sendMessage(tabId, {
                    status: 'success',
                    opted_in: true,
                    cookies: data.cookies,
                    domain: domain
                  });
                } else {
                  chrome.tabs.sendMessage(tabId, {
                    status: 'error',
                    message: 'User has not opted in'
                  });
                }
              });
            })
          .catch(error => {
            chrome.tabs.sendMessage(tabId, {
              status: 'error',
              message: error.message
            });
          });
        });
      } catch (error) {
        chrome.tabs.sendMessage(tabId, {
          status: 'error',
          message: error.message
        });
      });
    }
  });
});

// Open options page
chrome.runtime.openOptionsPage = (reason) => {
  chrome.runtime.openOptionsPage((reason) => {
    console.log(`Options page closed: ${reason}`);
  });
});

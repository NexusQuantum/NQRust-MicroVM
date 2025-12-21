// Table of Contents Collapse/Uncollapse functionality
(function() {
  'use strict';

  // Wait for DOM to be ready
  document.addEventListener('DOMContentLoaded', function() {
    // Find the TOC container
    const tocContainer = document.querySelector('.td-toc');

    if (!tocContainer) {
      return; // No TOC on this page
    }

    // Create collapse button
    const collapseBtn = document.createElement('button');
    collapseBtn.className = 'toc-collapse-btn';
    collapseBtn.setAttribute('aria-label', 'Toggle table of contents');
    collapseBtn.innerHTML = `
      <svg class="toc-icon toc-icon-collapse" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M4 8a.5.5 0 0 1 .5-.5h7a.5.5 0 0 1 0 1h-7A.5.5 0 0 1 4 8z"/>
      </svg>
      <svg class="toc-icon toc-icon-expand" width="16" height="16" viewBox="0 0 16 16" fill="currentColor" style="display:none;">
        <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/>
      </svg>
    `;

    // Find TOC header
    const tocHeader = tocContainer.querySelector('.td-toc__header, h2, .toc-title');

    if (tocHeader) {
      // Add button next to header
      tocHeader.style.display = 'flex';
      tocHeader.style.justifyContent = 'space-between';
      tocHeader.style.alignItems = 'center';
      tocHeader.appendChild(collapseBtn);
    } else {
      // If no header found, create one
      const header = document.createElement('div');
      header.className = 'td-toc__header';
      header.style.display = 'flex';
      header.style.justifyContent = 'space-between';
      header.style.alignItems = 'center';
      header.innerHTML = '<h2 style="margin:0;">On This Page</h2>';
      header.appendChild(collapseBtn);
      tocContainer.insertBefore(header, tocContainer.firstChild);
    }

    // Get TOC content
    const tocContent = tocContainer.querySelector('.td-toc__content, nav, ul');

    if (!tocContent) {
      return;
    }

    // Check localStorage for saved state
    const savedState = localStorage.getItem('toc-collapsed');
    let isCollapsed = savedState === 'true';

    // Apply initial state
    if (isCollapsed) {
      tocContent.style.display = 'none';
      collapseBtn.querySelector('.toc-icon-collapse').style.display = 'none';
      collapseBtn.querySelector('.toc-icon-expand').style.display = 'block';
      tocContainer.classList.add('toc-collapsed');
    }

    // Toggle function
    function toggleTOC() {
      isCollapsed = !isCollapsed;

      if (isCollapsed) {
        tocContent.style.display = 'none';
        collapseBtn.querySelector('.toc-icon-collapse').style.display = 'none';
        collapseBtn.querySelector('.toc-icon-expand').style.display = 'block';
        tocContainer.classList.add('toc-collapsed');
      } else {
        tocContent.style.display = 'block';
        collapseBtn.querySelector('.toc-icon-collapse').style.display = 'block';
        collapseBtn.querySelector('.toc-icon-expand').style.display = 'none';
        tocContainer.classList.remove('toc-collapsed');
      }

      // Save state to localStorage
      localStorage.setItem('toc-collapsed', isCollapsed);
    }

    // Add click event
    collapseBtn.addEventListener('click', toggleTOC);
  });
})();

// Icons are inline SVG drawn with currentColor, so they follow the tokens
// and never load an image file or a remote URL.

export function InboxIcon() {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        d="M3.5 13.5 6 5.5h12l2.5 8v5h-17zM3.5 13.5h5l1.5 2.5h4l1.5-2.5h5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
    </svg>
  );
}

import { useState } from 'react';
import type { Feed } from '../types';

/** Derive a colour from the feed title for the initial-circle fallback. */
function avatarColor(title: string): string {
  const colours = [
    '#e05d5d', '#e07a5d', '#e0a35d', '#d4c050',
    '#6ab04c', '#4caf88', '#4ca8af', '#4c82e0',
    '#7b5de0', '#c45de0', '#e05da7',
  ];
  let hash = 0;
  for (let i = 0; i < title.length; i++) hash = title.charCodeAt(i) + ((hash << 5) - hash);
  return colours[Math.abs(hash) % colours.length];
}

interface FeedAvatarProps {
  feed: Feed;
  className?: string;
}

export function FeedAvatar({ feed, className = 'feed-avatar' }: FeedAvatarProps) {
  const [imgFailed, setImgFailed] = useState(false);
  const initial = (feed.title.trim()[0] ?? '?').toUpperCase();

  if (feed.icon_url && !imgFailed) {
    return (
      <img
        className={className}
        src={feed.icon_url}
        alt=""
        onError={() => setImgFailed(true)}
      />
    );
  }
  return (
    <span
      className={`${className} feed-avatar--initial`}
      style={{ background: avatarColor(feed.title) }}
    >
      {initial}
    </span>
  );
}

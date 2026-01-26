/**
 * Session Sound Notification Hook
 * Plays a sound when sessions end or user input is required
 */

const playSound = (soundType = 'default') => {
  // For macOS, we can use the 'afplay' command to play system sounds
  const sounds = {
    default: '/System/Library/Sounds/Ping.aiff',
    complete: '/System/Library/Sounds/Glass.aiff',
    attention: '/System/Library/Sounds/Sosumi.aiff',
    error: '/System/Library/Sounds/Basso.aiff'
  };
  
  const soundFile = sounds[soundType] || sounds.default;
  
  try {
    // Use afplay to play the sound on macOS
    require('child_process').exec(`afplay "${soundFile}"`, (error) => {
      if (error) {
        console.log('Could not play sound:', error.message);
      }
    });
  } catch (err) {
    console.log('Sound playback failed:', err.message);
  }
};

module.exports = {
  onSessionEnd: () => {
    console.log('🔊 Session completed - playing completion sound');
    playSound('complete');
  },
  
  onUserInputRequired: () => {
    console.log('🔊 User input needed - playing attention sound');
    playSound('attention');
  },
  
  onError: () => {
    console.log('🔊 Error occurred - playing error sound');
    playSound('error');
  }
};
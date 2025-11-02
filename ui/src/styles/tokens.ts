/**
 * 设计令牌 (Design Tokens)
 * 统一的设计系统变量，确保整个应用的视觉一致性
 */

export const DesignTokens = {
  // 间距系统 (8px 基础)
  spacing: {
    xs: 4,
    sm: 8,
    md: 12,
    lg: 16,
    xl: 24,
    xxl: 32,
  },

  // 圆角
  borderRadius: {
    sm: 4,
    md: 6,
    lg: 8,
    xl: 12,
  },

  // 字体大小
  fontSize: {
    xs: 12,
    sm: 14,
    md: 16,
    lg: 18,
    xl: 20,
    xxl: 24,
  },

  // 阴影 - 使用 CSS 变量支持主题切换
  shadow: {
    none: 'none',
    sm: 'var(--custom-shadow-sm)',
    md: 'var(--custom-shadow-md)',
    lg: 'var(--custom-shadow-lg)',
    xl: 'var(--custom-shadow-xl)',
  },

  // 过渡动画
  transition: {
    fast: '0.1s cubic-bezier(0.4, 0, 0.2, 1)',
    normal: '0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    slow: '0.3s cubic-bezier(0.4, 0, 0.2, 1)',
  },

  // 尺寸
  size: {
    sidebarMin: 200,
    sidebarMax: 600,
    sidebarDefault: 280,
    headerHeight: 56,
    dragHandleSize: 8,
  },
} as const;

export type Spacing = keyof typeof DesignTokens.spacing;
export type BorderRadius = keyof typeof DesignTokens.borderRadius;
export type FontSize = keyof typeof DesignTokens.fontSize;
export type Shadow = keyof typeof DesignTokens.shadow;

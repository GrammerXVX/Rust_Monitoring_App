import React, { useEffect, useState, createContext, useContext } from 'react'
type Theme = 'system' | 'dark' | 'light'
interface ThemeContextType {
  theme: Theme
  setTheme: (theme: Theme) => void
  currentTheme: 'dark' | 'light'
}
const ThemeContext = createContext<ThemeContextType | undefined>(undefined)
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setTheme] = useState<Theme>('system')
  const [currentTheme, setCurrentTheme] = useState<'dark' | 'light'>('dark')
  useEffect(() => {
    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
      setCurrentTheme(mediaQuery.matches ? 'dark' : 'light')
      const handler = (e: MediaQueryListEvent) => {
        setCurrentTheme(e.matches ? 'dark' : 'light')
      }
      mediaQuery.addEventListener('change', handler)
      return () => mediaQuery.removeEventListener('change', handler)
    } else {
      setCurrentTheme(theme as 'dark' | 'light')
    }
  }, [theme])
  useEffect(() => {
    const root = document.documentElement
    root.classList.remove('light', 'dark')
    root.classList.add(currentTheme)
  }, [currentTheme])
  return (
    <ThemeContext.Provider
      value={{
        theme,
        setTheme,
        currentTheme,
      }}
    >
      {children}
    </ThemeContext.Provider>
  )
}
export const useTheme = () => {
  const context = useContext(ThemeContext)
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider')
  }
  return context
}

import * as React from "react"
import { cn } from "@/lib/utils"

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  icon?: React.ReactNode
  error?: string
  helperText?: string
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, icon, error, helperText, ...props }, ref) => {
    return (
      <div className="w-full">
        <div className="relative">
          {icon && (
            <div className="absolute left-0 top-0 bottom-0 w-12 flex items-center justify-center pointer-events-none z-10 text-gray-500">
              {icon}
            </div>
          )}
          <input
            type={type}
            className={cn(
              // Base styles
              "flex w-full rounded-xl border bg-gray-800/50 px-4 py-4 text-base text-white",
              "placeholder:text-gray-500",
              "focus:outline-none focus:ring-2 focus:ring-blue-500/20",
              "disabled:cursor-not-allowed disabled:opacity-50",
              "transition-all duration-200",
              
              // Padding adjustments for icon
              icon ? "pl-12" : "pl-4",
              
              // Border styles based on error state
              error 
                ? "border-red-500 focus:border-red-500" 
                : "border-gray-700 focus:border-blue-500",
              
              className
            )}
            ref={ref}
            {...props}
          />
          {error && (
            <div className="absolute right-0 top-0 bottom-0 w-12 flex items-center justify-center pointer-events-none text-red-500">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" x2="12" y1="8" y2="12"/>
                <line x1="12" x2="12.01" y1="16" y2="16"/>
              </svg>
            </div>
          )}
        </div>
        {helperText && !error && (
          <p className="mt-1.5 text-xs text-gray-500">{helperText}</p>
        )}
        {error && (
          <p className="mt-1.5 text-xs text-red-400">{error}</p>
        )}
      </div>
    )
  }
)
Input.displayName = "Input"

export { Input }

"use client";

export default function OfflinePage() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-[#0a0f1c] text-white px-4">
      <div className="text-center max-w-md">
        {/* Logo / Brand */}
        <div className="mb-8">
          <div className="w-16 h-16 mx-auto rounded-2xl bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center mb-4">
            <span className="text-2xl font-bold">IO</span>
          </div>
          <h1 className="text-3xl font-bold bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
            Investor OS
          </h1>
        </div>

        {/* Offline icon */}
        <div className="mb-6">
          <svg
            className="w-20 h-20 mx-auto text-gray-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={1.5}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
            />
          </svg>
        </div>

        {/* Message */}
        <h2 className="text-xl font-semibold text-gray-200 mb-3">
          You are offline
        </h2>
        <p className="text-gray-400 mb-8">
          Please check your internet connection and try again. Market data and
          trading features require an active connection.
        </p>

        {/* Retry button */}
        <button
          onClick={() => window.location.reload()}
          className="px-6 py-3 bg-purple-600 hover:bg-purple-700 text-white rounded-lg font-medium transition-colors"
        >
          Retry Connection
        </button>
      </div>
    </div>
  );
}

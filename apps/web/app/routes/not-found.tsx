import { Link } from 'react-router'
import { Button } from '@revlentless/ui/components/button'

export default function NotFound() {
  return (
    <div className="mx-auto max-w-xl py-20 text-center">
      <h1 className="text-3xl font-bold">Page not found</h1>
      <p className="mt-2 text-muted-foreground">The page you’re looking for doesn’t exist.</p>
      <div className="mt-6">
        <Button asChild>
          <Link to="/">Go home</Link>
        </Button>
      </div>
    </div>
  )
}

/**
 * Glassmorphism Trust Hero — from https://21st.dev/r/easemize/glassmorphism-trust-hero
 *
 * Saved directly instead of using `npx shadcn add` because:
 *  - this project uses Tailwind v4 (no tailwind.config.js / no components.json)
 *  - shadcn init would have overwritten our /lib/utils.ts and theme
 *
 * The component has no registry dependencies — only `lucide-react`, which is already installed.
 * Original CC0 / MIT per 21st.dev. Background image swapped for a CSS gradient.
 */
import {
  ArrowRight,
  Play,
  Target,
  Crown,
  Star,
  Hexagon,
  Triangle,
  Command,
  Ghost,
  Gem,
  Cpu,
} from 'lucide-react'

const CLIENTS = [
  { name: 'Acme Corp', icon: Hexagon },
  { name: 'Quantum', icon: Triangle },
  { name: 'Command+Z', icon: Command },
  { name: 'Phantom', icon: Ghost },
  { name: 'Ruby', icon: Gem },
  { name: 'Chipset', icon: Cpu },
]

const StatItem = ({ value, label }: { value: string; label: string }) => (
  <div className="flex flex-col items-center justify-center transition-transform hover:-translate-y-1 cursor-default">
    <span className="text-xl font-bold text-white sm:text-2xl">{value}</span>
    <span className="text-[10px] uppercase tracking-wider text-zinc-500 font-medium sm:text-xs">
      {label}
    </span>
  </div>
)

export default function GlassmorphismTrustHero() {
  return (
    <div className="relative w-full overflow-hidden bg-zinc-950 font-sans text-white">
      <style>{`
        @keyframes fadeSlideIn { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
        @keyframes marquee { from { transform: translateX(0); } to { transform: translateX(-50%); } }
        .gth-fade-in { animation: fadeSlideIn 0.8s ease-out forwards; opacity: 0; }
        .gth-marquee { animation: marquee 40s linear infinite; }
        .gth-d-100 { animation-delay: 0.1s; }
        .gth-d-200 { animation-delay: 0.2s; }
        .gth-d-300 { animation-delay: 0.3s; }
        .gth-d-400 { animation-delay: 0.4s; }
        .gth-d-500 { animation-delay: 0.5s; }
      `}</style>

      {/* Backdrop: subtle radial + grid */}
      <div
        aria-hidden
        className="absolute inset-0 z-0"
        style={{
          background:
            'radial-gradient(900px 500px at 70% 10%, rgba(255,205,117,0.10), transparent 60%), radial-gradient(700px 400px at 10% 60%, rgba(255,255,255,0.04), transparent 60%)',
          maskImage: 'linear-gradient(180deg, transparent, black 0%, black 70%, transparent)',
          WebkitMaskImage:
            'linear-gradient(180deg, transparent, black 0%, black 70%, transparent)',
        }}
      />

      <div className="relative z-10 mx-auto max-w-7xl px-4 pt-24 pb-12 sm:px-6 md:pt-32 md:pb-20 lg:px-8">
        <div className="grid grid-cols-1 items-start gap-12 lg:grid-cols-12 lg:gap-8">
          {/* LEFT */}
          <div className="flex flex-col justify-center space-y-8 pt-8 lg:col-span-7">
            <div className="gth-fade-in gth-d-100">
              <div className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1.5 backdrop-blur-md transition-colors hover:bg-white/10">
                <span className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-300 sm:text-xs">
                  Award-Winning Design
                  <Star className="h-3.5 w-3.5 fill-yellow-400 text-yellow-400" />
                </span>
              </div>
            </div>

            <h1
              className="gth-fade-in gth-d-200 text-5xl font-medium leading-[0.9] tracking-tighter sm:text-6xl lg:text-7xl xl:text-8xl"
              style={{
                maskImage: 'linear-gradient(180deg, black 0%, black 80%, transparent 100%)',
                WebkitMaskImage:
                  'linear-gradient(180deg, black 0%, black 80%, transparent 100%)',
              }}
            >
              Crafting Digital
              <br />
              <span className="bg-gradient-to-br from-white via-white to-[#ffcd75] bg-clip-text text-transparent">
                Experiences
              </span>
              <br />
              That Matter
            </h1>

            <p className="gth-fade-in gth-d-300 max-w-xl text-lg leading-relaxed text-zinc-400">
              We design interfaces that combine beauty with functionality, creating seamless
              experiences that users love and businesses thrive on.
            </p>

            <div className="gth-fade-in gth-d-400 flex flex-col gap-4 sm:flex-row">
              <button className="group inline-flex cursor-pointer items-center justify-center gap-2 rounded-full bg-white px-8 py-4 text-sm font-semibold text-zinc-950 transition-all hover:scale-[1.02] hover:bg-zinc-200 active:scale-[0.98]">
                View Portfolio
                <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
              </button>

              <button className="group inline-flex cursor-pointer items-center justify-center gap-2 rounded-full border border-white/10 bg-white/5 px-8 py-4 text-sm font-semibold text-white backdrop-blur-sm transition-colors hover:border-white/20 hover:bg-white/10">
                <Play className="h-4 w-4 fill-current" />
                Watch Showreel
              </button>
            </div>
          </div>

          {/* RIGHT */}
          <div className="space-y-6 lg:col-span-5 lg:mt-12">
            <div className="gth-fade-in gth-d-500 relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 p-8 shadow-2xl backdrop-blur-xl">
              <div
                aria-hidden
                className="pointer-events-none absolute -top-16 -right-16 h-64 w-64 rounded-full bg-white/5 blur-3xl"
              />

              <div className="relative z-10">
                <div className="mb-8 flex items-center gap-4">
                  <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/10 ring-1 ring-white/20">
                    <Target className="h-6 w-6 text-white" />
                  </div>
                  <div>
                    <div className="text-3xl font-bold tracking-tight text-white">150+</div>
                    <div className="text-sm text-zinc-400">Projects Delivered</div>
                  </div>
                </div>

                <div className="mb-8 space-y-3">
                  <div className="flex justify-between text-sm">
                    <span className="text-zinc-400">Client Satisfaction</span>
                    <span className="font-medium text-white">98%</span>
                  </div>
                  <div className="h-2 w-full overflow-hidden rounded-full bg-zinc-800/50">
                    <div className="h-full w-[98%] rounded-full bg-gradient-to-r from-white to-zinc-400" />
                  </div>
                </div>

                <div className="mb-6 h-px w-full bg-white/10" />

                <div className="grid grid-cols-3 gap-4 text-center">
                  <StatItem value="5+" label="Years" />
                  <div className="mx-auto h-full w-px bg-white/10" />
                  <StatItem value="24/7" label="Support" />
                  <div className="mx-auto h-full w-px bg-white/10" />
                  <StatItem value="100%" label="Quality" />
                </div>

                <div className="mt-8 flex flex-wrap gap-2">
                  <div className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-[10px] font-medium tracking-wide text-zinc-300">
                    <span className="relative flex h-2 w-2">
                      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" />
                      <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
                    </span>
                    ACTIVE
                  </div>
                  <div className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-[10px] font-medium tracking-wide text-zinc-300">
                    <Crown className="h-3 w-3 text-yellow-500" />
                    PREMIUM
                  </div>
                </div>
              </div>
            </div>

            {/* Marquee */}
            <div className="gth-fade-in gth-d-500 relative overflow-hidden rounded-3xl border border-white/10 bg-white/5 py-8 backdrop-blur-xl">
              <h3 className="mb-6 px-8 text-sm font-medium text-zinc-400">
                Trusted by Industry Leaders
              </h3>
              <div
                className="relative flex overflow-hidden"
                style={{
                  maskImage:
                    'linear-gradient(to right, transparent, black 20%, black 80%, transparent)',
                  WebkitMaskImage:
                    'linear-gradient(to right, transparent, black 20%, black 80%, transparent)',
                }}
              >
                <div className="gth-marquee flex gap-12 whitespace-nowrap px-4">
                  {[...CLIENTS, ...CLIENTS, ...CLIENTS].map((client, i) => (
                    <div
                      key={i}
                      className="flex cursor-default items-center gap-2 opacity-50 grayscale transition-all hover:scale-105 hover:opacity-100 hover:grayscale-0"
                    >
                      <client.icon className="h-6 w-6 fill-current text-white" />
                      <span className="text-lg font-bold tracking-tight text-white">
                        {client.name}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// Same-WASM-ABI regression for the browser resource-budget clock.
#include <sys/resource.h>
#include <emscripten.h>
#include <cstdio>
#include <cerrno>
int main() {
    rusage before{},after{},children{};
    if(getrusage(RUSAGE_SELF,&before))return 1;
    const double start=emscripten_get_now();
    while(emscripten_get_now()-start<35.0) {}
    if(getrusage(RUSAGE_SELF,&after))return 2;
    const double delta=double(after.ru_utime.tv_sec-before.ru_utime.tv_sec)+double(after.ru_utime.tv_usec-before.ru_utime.tv_usec)/1e6;
    if(delta<0.025 || delta>2 || after.ru_maxrss<=0)return 3;
    if(getrusage(RUSAGE_CHILDREN,&children) || children.ru_utime.tv_sec || children.ru_utime.tv_usec)return 4;
    if(getrusage(12345,&after)!=-1 || errno!=EINVAL)return 5;
    std::printf("WASM monotonic resource clock PASS (%.3f s)\n",delta);
}

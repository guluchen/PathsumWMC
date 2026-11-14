// Independent GMP rational differential oracle for the optimized adapter. MIT.
#include "ComplexMPFR.h"
#include <cassert>
#include <sstream>
#include <iostream>
struct Ref {mpq_class r,i;};
static Ref add(const Ref&a,const Ref&b){return {a.r+b.r,a.i+b.i};}
static Ref mul(const Ref&a,const Ref&b){return {a.r*b.r-a.i*b.i,a.r*b.i+a.i*b.r};}
static void check(const Complex& z,const Ref& q){std::ostringstream a,b;a<<z;b<<q.r<<(q.i<0?"":"+")<<q.i<<"i";assert(a.str()==b.str());}
int main(){
 uint64_t seed=0x38298fae;auto next=[&](){seed=seed*6364136223846793005ULL+1;return seed>>32;};
 unsigned checks=0;
 for(unsigned trial=0;trial<500;++trial){
  Complex value(1);Ref ref{1,0};
  for(unsigned step=0;step<24;++step){
   mpz_class d=mpz_class(1)<<(next()%192);if(trial%7==0)d*=3;
   mpq_class r(mpz_class(static_cast<long>(next()%31)-15),d),i(mpz_class(static_cast<long>(next()%31)-15),d);r.canonicalize();i.canonicalize();
   Complex b(r,i);Ref br{r,i};check(b,br);++checks;
   if(next()&1){value+=b;ref=add(ref,br);}else{value*=b;ref=mul(ref,br);}
   check(value,ref);++checks;
   if(step%6==0){int n=static_cast<int>(next()%20)-10;value=value<<n;mpq_class k=n>=0?mpq_class(mpz_class(1)<<n):mpq_class(mpz_class(1),mpz_class(1)<<(-n));ref={ref.r*k,ref.i*k};check(value,ref);++checks;}
  }
 }
 Complex z("-3/7","2/9");z*=z;check(z,mul({mpq_class(-3,7),mpq_class(2,9)},{mpq_class(-3,7),mpq_class(2,9)}));
 assert(Complex(0)==0);assert(Complex(1)==1);assert((Complex("1/2","0")<<1)==1);
 std::cout<<"Exact dyadic/fallback differential PASS "<<checks<<" checks\n";
}

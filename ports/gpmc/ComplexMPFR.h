// PathsumWMC exact complex arithmetic for GPMC. MIT licensed.
// Dyadic values use integer mantissas with a shared power-of-two denominator.
// General rational inputs retain an exact GMP rational fallback. No rounding.
#ifndef PATHSUM_GPMC_COMPLEX_H
#define PATHSUM_GPMC_COMPLEX_H
#include <gmpxx.h>
#include <algorithm>
#include <cstdint>
#include <limits>
#include <memory>
#include <ostream>
#include <stdexcept>
#include <string>
#include <utility>
class Complex {
    struct Q {mpq_class r,i;Q(const mpq_class& a,const mpq_class& b):r(a),i(b){}};
    mpz_class re_,im_;
    int64_t exponent_=0; // value = (re_ + i*im_) * 2^(-exponent_)
    std::shared_ptr<const Q> general_;
    static int64_t add_exp(int64_t a,int64_t b) {
        if((b>0 && a>std::numeric_limits<int64_t>::max()-b) || (b<0 && a<std::numeric_limits<int64_t>::min()-b))throw std::overflow_error("exact exponent capacity exceeded");
        return a+b;
    }
    static void shift(mpz_class& n,uint64_t bits) {
        if(bits>std::numeric_limits<mp_bitcnt_t>::max())throw std::overflow_error("exact shift capacity exceeded");
        mpz_mul_2exp(n.get_mpz_t(),n.get_mpz_t(),static_cast<mp_bitcnt_t>(bits));
    }
    static int64_t denominator_bits(const mpq_class& n) {
        if(mpz_popcount(n.get_den_mpz_t())!=1)return -1;
        const auto k=mpz_scan1(n.get_den_mpz_t(),0);
        if(k>static_cast<uint64_t>(std::numeric_limits<int64_t>::max()))throw std::overflow_error("exact denominator capacity exceeded");
        return static_cast<int64_t>(k);
    }
    void normalize() {
        if(re_==0 && im_==0){exponent_=0;return;}
        auto a=re_==0?std::numeric_limits<mp_bitcnt_t>::max():mpz_scan1(re_.get_mpz_t(),0);
        auto b=im_==0?std::numeric_limits<mp_bitcnt_t>::max():mpz_scan1(im_.get_mpz_t(),0);
        auto k=std::min(a,b);
        if(k>static_cast<uint64_t>(std::numeric_limits<int64_t>::max()))throw std::overflow_error("exact normalization capacity exceeded");
        if(k){mpz_tdiv_q_2exp(re_.get_mpz_t(),re_.get_mpz_t(),k);mpz_tdiv_q_2exp(im_.get_mpz_t(),im_.get_mpz_t(),k);exponent_=add_exp(exponent_,-static_cast<int64_t>(k));}
    }
    static mpq_class fraction(const mpz_class& n,int64_t e) {
        mpz_class a=n,d=1;
        if(e>=0)shift(d,static_cast<uint64_t>(e));
        else shift(a,static_cast<uint64_t>(-(e+1))+1);
        mpq_class q(a,d);q.canonicalize();return q;
    }
    Q rational() const {return general_?*general_:Q(fraction(re_,exponent_),fraction(im_,exponent_));}
    static Complex dyadic(mpz_class r,mpz_class i,int64_t e) {Complex z;z.re_=std::move(r);z.im_=std::move(i);z.exponent_=e;z.normalize();return z;}
public:
    Complex():re_(1),im_(0){}
    Complex(int r):re_(r),im_(0){normalize();}
    Complex(const std::string& r):Complex(mpq_class(r),mpq_class(0)){}
    Complex(const std::string& r,const std::string& i):Complex(mpq_class(r),mpq_class(i)){}
    Complex(const mpq_class& r,const mpq_class& i):re_(0),im_(0) {
        mpq_class a=r,b=i;a.canonicalize();b.canonicalize();
        auto x=denominator_bits(a),y=denominator_bits(b);
        if(x<0 || y<0){general_=std::make_shared<const Q>(a,b);return;}
        exponent_=std::max(x,y);re_=a.get_num();im_=b.get_num();
        shift(re_,static_cast<uint64_t>(exponent_-x));shift(im_,static_cast<uint64_t>(exponent_-y));normalize();
    }
    Complex& operator=(int r){*this=Complex(r);return *this;}
    Complex& operator=(const std::string& r){*this=Complex(r);return *this;}
    bool operator==(const Complex& z) const {
        if(!general_ && !z.general_)return exponent_==z.exponent_ && re_==z.re_ && im_==z.im_;
        auto a=rational(),b=z.rational();return a.r==b.r && a.i==b.i;
    }
    bool operator!=(const Complex& z) const{return !(*this==z);}
    bool operator==(int r)const{return *this==Complex(r);}
    bool operator!=(int r)const{return !(*this==r);}
    Complex operator+(const Complex& z) const {
        if(general_ || z.general_){auto a=rational(),b=z.rational();return Complex(a.r+b.r,a.i+b.i);}
        const auto e=std::max(exponent_,z.exponent_);
        mpz_class a=re_,b=im_,c=z.re_,d=z.im_;
        shift(a,static_cast<uint64_t>(e)-static_cast<uint64_t>(exponent_));shift(b,static_cast<uint64_t>(e)-static_cast<uint64_t>(exponent_));
        shift(c,static_cast<uint64_t>(e)-static_cast<uint64_t>(z.exponent_));shift(d,static_cast<uint64_t>(e)-static_cast<uint64_t>(z.exponent_));
        return dyadic(a+c,b+d,e);
    }
    Complex operator*(const Complex& z) const {
        if(general_ || z.general_){auto a=rational(),b=z.rational();return Complex(a.r*b.r-a.i*b.i,a.r*b.i+a.i*b.r);}
        return dyadic(re_*z.re_-im_*z.im_,re_*z.im_+im_*z.re_,add_exp(exponent_,z.exponent_));
    }
    Complex& operator+=(const Complex& z){*this=(*this)+z;return *this;}
    Complex& operator*=(const Complex& z){*this=(*this)*z;return *this;}
    Complex operator<<(int n) const {
        if(general_){auto a=rational();mpq_class k;if(n>=0)k=mpq_class(mpz_class(1)<<n);else k=mpq_class(mpz_class(1),mpz_class(1)<<(-static_cast<int64_t>(n)));return Complex(a.r*k,a.i*k);}
        return dyadic(re_,im_,add_exp(exponent_,-static_cast<int64_t>(n)));
    }
    friend std::ostream& operator<<(std::ostream& os,const Complex& z){auto a=z.rational();return os<<a.r<<(a.i<0?"":"+")<<a.i<<"i";}
};
#endif

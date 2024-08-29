pragma solidity ^0.8.0;

contract benchmark {
    uint256 value;
    constructor() {
      value = fac(57);
    }

    function get_calculation() public view returns (uint256) {
        return value;
    }

    function fac(uint n) internal pure returns(uint256) { 
        uint256 r = 1;
        for (uint256 k = 2; k <= n; k++) {
            r = k * r;
        }

        return r;
    }
}


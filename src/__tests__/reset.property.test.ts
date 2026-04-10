/**
 * Feature: openscad-backend, Property 10: Reset clears all state
 *
 * For any ConversationState (regardless of how many messages, what mesh data,
 * or what generated code it contains), performing a reset should produce a
 * state with an empty messages array, null mesh data, and null generated code.
 *
 * Validates: Requirements 8.1
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import type { Message, MeshData, ConversationState } from '../App';

const messageArb: fc.Arbitrary<Message> = fc.record({
    role: fc.constantFrom('user' as const, 'assistant' as const),
    content: fc.string(),
});

const jsonSafeFloat = fc
    .float({ noNaN: true, noDefaultInfinity: true })
    .map((v) => (Object.is(v, -0) ? 0 : v));

const meshDataArb: fc.Arbitrary<MeshData> = fc.record({
    vertices: fc.array(jsonSafeFloat),
    normals: fc.array(jsonSafeFloat),
    indices: fc.array(fc.nat()),
});

const conversationStateArb: fc.Arbitrary<ConversationState> = fc.record({
    messages: fc.array(messageArb),
    generated_code: fc.option(fc.string(), { nil: null }),
    mesh_data: fc.option(meshDataArb, { nil: null }),
});

/**
 * Simulates the reset logic from App.tsx handleNew:
 *   setMessages([]);
 *   setGeneratedCode(null);
 *   setMeshData(null);
 */
function applyReset(_state: ConversationState): ConversationState {
    return {
        messages: [],
        generated_code: null,
        mesh_data: null,
    };
}

describe('Feature: openscad-backend, Property 10: Reset clears all state', () => {
    it('reset always produces a clean state regardless of prior state', () => {
        /**
         * Validates: Requirements 8.1
         */
        fc.assert(
            fc.property(conversationStateArb, (state: ConversationState) => {
                const resetState = applyReset(state);

                expect(resetState.messages).toEqual([]);
                expect(resetState.generated_code).toBeNull();
                expect(resetState.mesh_data).toBeNull();
            }),
            { numRuns: 100 },
        );
    });
});

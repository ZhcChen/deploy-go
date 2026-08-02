//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'csrf_token_response.g.dart';

/// CsrfTokenResponse
///
/// Properties:
/// * [csrfToken]
@BuiltValue()
abstract class CsrfTokenResponse implements Built<CsrfTokenResponse, CsrfTokenResponseBuilder> {
  @BuiltValueField(wireName: r'csrf_token')
  String get csrfToken;

  CsrfTokenResponse._();

  factory CsrfTokenResponse([void updates(CsrfTokenResponseBuilder b)]) = _$CsrfTokenResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CsrfTokenResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CsrfTokenResponse> get serializer => _$CsrfTokenResponseSerializer();
}

class _$CsrfTokenResponseSerializer implements PrimitiveSerializer<CsrfTokenResponse> {
  @override
  final Iterable<Type> types = const [CsrfTokenResponse, _$CsrfTokenResponse];

  @override
  final String wireName = r'CsrfTokenResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CsrfTokenResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'csrf_token';
    yield serializers.serialize(
      object.csrfToken,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CsrfTokenResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CsrfTokenResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'csrf_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.csrfToken = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CsrfTokenResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CsrfTokenResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
